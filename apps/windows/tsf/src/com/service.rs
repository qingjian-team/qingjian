//! 文本服务对象：[`ITfTextInputProcessor`]（激活 / 停用）与 [`ITfKeyEventSink`]（收键）。

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{E_INVALIDARG, FALSE, LPARAM, WPARAM};
use windows::Win32::UI::TextServices::{
    IEnumTfDisplayAttributeInfo, ITfContext, ITfDisplayAttributeInfo, ITfDisplayAttributeProvider,
    ITfDisplayAttributeProvider_Impl, ITfKeyEventSink, ITfKeyEventSink_Impl, ITfKeystrokeMgr,
    ITfLangBarItemButton, ITfLangBarItemMgr, ITfTextInputProcessor, ITfTextInputProcessor_Impl,
    ITfThreadMgr,
};
use windows::core::{BOOL, GUID, IUnknownImpl, Interface, Ref, Result, implement};

use qingjian_platform::protocol::{KeyEvent, KeyOutcome, SessionId};

use super::candidates::CandidateWindow;
use super::composition::{Shared, preedit_string};
use super::keys;
use super::langbar::{ModeButton, ModeState};
use super::log::log;
use super::poll::PollTimer;
use crate::client::EngineClient;
use crate::client::pipe::{PipeStream, connect_default};

/// 连 Server 的会话客户端，与云联想轮询定时器共享（STA 单线程，不并发）。连不上时为 `None`，键照样放行。
pub(crate) type SharedClient = Rc<RefCell<Option<EngineClient<PipeStream>>>>;

/// 连不上 Server 后隔多久再试。每次尝试连接都在应用的 UI 线程上，不能每键都试。
const RECONNECT_INTERVAL: Duration = Duration::from_secs(2);

/// 一个 TSF 文本服务实例（每线程一个）。
#[implement(ITfTextInputProcessor, ITfKeyEventSink, ITfDisplayAttributeProvider)]
pub struct TextService {
    /// 激活时拿到的线程管理器，停用时用它反注册击键 sink。
    thread_mgr: RefCell<Option<ITfThreadMgr>>,

    /// TSF 分配的 client id。
    client_id: Cell<u32>,

    /// 引擎层。
    engine: SharedClient,

    /// 跨按键存活的组句状态，与编辑会话 / 组句 sink / 定时器共享。
    shared: Rc<Shared>,

    /// 云联想轮询定时器；挂失败时为 `None`，退化为只在按键时收云结果。
    poll_timer: RefCell<Option<PollTimer>>,

    /// 上次连 Server 失败的时间；没连上时按键与获得焦点都会隔 [`RECONNECT_INTERVAL`] 再试。
    last_connect_failure: Cell<Option<Instant>>,

    /// 持久的英文模式（单击 Shift 翻转）。`false` 是中文模式。随每个按键带给 Server，也驱动中 / 英指示器。
    english_mode: Cell<bool>,

    /// 中 / 英 指示器的共享状态（与语言栏按钮共用）。
    mode_state: Rc<ModeState>,

    /// 登记在系统语言栏上的中 / 英按钮；停用时反注册。
    mode_button: RefCell<Option<ITfLangBarItemButton>>,
}

thread_local! {
    /// 本线程当前激活的文本服务，供 Shift 钩子回调切模式。`Activate` 设、`Deactivate`（卸钩子后）清。
    static ACTIVE_SERVICE: Cell<*const TextService_Impl> = const { Cell::new(core::ptr::null()) };
}

/// Shift 钩子检测到一次单击时调（见 [`super::hook`]）：切当前激活文本服务的中英模式。
pub(super) fn on_shift_tap() {
    let service = ACTIVE_SERVICE.with(|s| s.get());
    if !service.is_null() {
        // SAFETY: 指针在 Activate 设、Deactivate 卸钩子后才清，期间 TextService 一直存活且在本线程。
        unsafe { (*service).toggle_english_mode() };
    }
}

impl TextService {
    #[allow(clippy::new_without_default)] // 有 lock_module 副作用
    pub fn new() -> Self {
        super::lock_module();
        Self {
            thread_mgr: RefCell::new(None),
            client_id: Cell::new(0),
            engine: Rc::new(RefCell::new(None)),
            shared: Shared::new(),
            poll_timer: RefCell::new(None),
            last_connect_failure: Cell::new(None),
            english_mode: Cell::new(false),
            mode_state: ModeState::new(),
            mode_button: RefCell::new(None),
        }
    }
}

impl Drop for TextService {
    fn drop(&mut self) {
        super::unlock_module();
    }
}

impl ITfTextInputProcessor_Impl for TextService_Impl {
    fn Activate(&self, ptim: Ref<ITfThreadMgr>, tid: u32) -> Result<()> {
        let thread_mgr = ptim.ok()?.clone();
        let keystroke: ITfKeystrokeMgr = thread_mgr.cast()?;
        let sink: ITfKeyEventSink = self.to_interface();
        // SAFETY: keystroke 有效；sink 是本对象的接口。
        unsafe { keystroke.AdviseKeyEventSink(tid, &sink, true)? };

        self.client_id.set(tid);
        // 下面三样都不致命：连不上 Server 键照样放行（之后按键时重连），没候选窗就没 UI，没定时器就只在按键时收云结果。
        self.connect();
        match CandidateWindow::new() {
            Ok(window) => self.shared.set_window(Some(window)),
            Err(error) => log(&format!("建候选窗口失败: {error}")),
        }
        match PollTimer::new(self.engine.clone(), self.shared.clone()) {
            Ok(timer) => *self.poll_timer.borrow_mut() = Some(timer),
            Err(error) => log(&format!("挂云联想轮询定时器失败: {error}")),
        }

        *self.thread_mgr.borrow_mut() = Some(thread_mgr);
        // 激活时从中文模式起步：登记语言栏中 / 英按钮，同步指示器。
        self.english_mode.set(false);
        self.add_lang_bar_item();
        self.update_mode_indicator();
        // 记下本服务并装 Shift 钩子（单击 Shift 切中英）。
        ACTIVE_SERVICE.with(|s| s.set(self as *const TextService_Impl));
        super::hook::install();
        log(&format!("青简 TSF 已激活 tid={tid}"));
        Ok(())
    }

    fn Deactivate(&self) -> Result<()> {
        // 先卸 Shift 钩子并清掉指针，之后回调不再碰本服务。
        super::hook::remove();
        ACTIVE_SERVICE.with(|s| s.set(core::ptr::null()));
        // 反注册语言栏中 / 英按钮（趁 thread_mgr 还在）。
        self.remove_lang_bar_item();
        // 停定时器，之后不再有回调碰 engine / shared。
        self.poll_timer.borrow_mut().take();
        // 切走输入法时敲了一半的拼音原样落定，再关会话。
        self.commit_pending();
        if let Some(thread_mgr) = self.thread_mgr.borrow_mut().take()
            && let Ok(keystroke) = thread_mgr.cast::<ITfKeystrokeMgr>()
        {
            // SAFETY: keystroke 有效。
            let _ = unsafe { keystroke.UnadviseKeyEventSink(self.client_id.get()) };
        }
        if let Some(client) = self.engine.borrow_mut().take() {
            let _ = client.close();
        }
        // 上屏的编辑会话已排队；剩下的句柄不再经编辑会话收尾，直接丢、销毁候选窗口。会话已关，过期标志一并作废。
        self.shared.reset();
        self.shared.take_server_stale();
        self.shared.set_window(None);
        log("青简 TSF 已停用");
        Ok(())
    }
}

impl ITfKeyEventSink_Impl for TextService_Impl {
    /// 获得焦点时顺手补一次连接：Server 起晚了、或中途重启过，切回来就能用，不必切走再切回输入法。
    /// 失去焦点时把敲了一半的拼音原样落定（对应 macOS 的 `commitComposition`），别让它跟着焦点跑到别的输入框。
    fn OnSetFocus(&self, fforeground: BOOL) -> Result<()> {
        if fforeground.as_bool() {
            self.ensure_connected();
            // 重新获得焦点时系统会重置输入指示器：刷一次中 / 英图标，否则要等下一次 Shift 才显示。
            self.update_mode_indicator();
        } else {
            self.commit_pending();
        }
        Ok(())
    }

    fn OnTestKeyDown(
        &self,
        _pic: Ref<ITfContext>,
        wparam: WPARAM,
        _lparam: LPARAM,
    ) -> Result<BOOL> {
        // 单击 Shift 切中英不走这里：击键 sink 收不到独立修饰键，改用 `super::hook` 的键盘钩子。
        Ok(self.would_eat(&self.key_event(wparam.0 as u32)).into())
    }

    fn OnKeyDown(&self, pic: Ref<ITfContext>, wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        let event = self.key_event(wparam.0 as u32);
        Ok(self.handle_key(pic, event).into())
    }

    fn OnTestKeyUp(&self, _pic: Ref<ITfContext>, _wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        Ok(FALSE)
    }

    fn OnKeyUp(&self, _pic: Ref<ITfContext>, _wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        Ok(FALSE)
    }

    fn OnPreservedKey(&self, _pic: Ref<ITfContext>, _rguid: *const GUID) -> Result<BOOL> {
        Ok(FALSE)
    }
}

/// 显示属性提供者：系统按 `GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER` 类别在本对象上查组句样式（内联下划线）。
impl ITfDisplayAttributeProvider_Impl for TextService_Impl {
    fn EnumDisplayAttributeInfo(&self) -> Result<IEnumTfDisplayAttributeInfo> {
        Ok(super::display_attribute::enumerator())
    }

    fn GetDisplayAttributeInfo(&self, guid: *const GUID) -> Result<ITfDisplayAttributeInfo> {
        // SAFETY: 系统传入的有效 GUID 指针。
        if unsafe { *guid } == super::display_attribute::GUID_DISPLAY_ATTRIBUTE_INPUT {
            Ok(super::display_attribute::info())
        } else {
            Err(E_INVALIDARG.into())
        }
    }
}

impl TextService_Impl {
    /// 连 Server 并开会话（会话 id 用 TSF 的 client id，带上宿主 exe 名）。失败记时间，供 [`Self::ensure_connected`] 退避。
    fn connect(&self) {
        let session = SessionId(self.client_id.get() as u64);
        let app = super::host_app_name();
        let connected = connect_default()
            .map_err(|e| e.to_string())
            .and_then(|stream| EngineClient::open(stream, session, app).map_err(|e| e.to_string()));
        match connected {
            Ok(client) => {
                *self.engine.borrow_mut() = Some(client);
                self.last_connect_failure.set(None);
                log("已连上 Server");
            }
            Err(error) => {
                self.last_connect_failure.set(Some(Instant::now()));
                log(&format!(
                    "连 Server 失败（qingjian-server 没起？）: {error}"
                ));
            }
        }
    }

    /// 没连上就重连一次，距上次失败不到 [`RECONNECT_INTERVAL`] 则跳过。返回此刻是否连着。
    fn ensure_connected(&self) -> bool {
        if self.engine.borrow().is_some() {
            return true;
        }
        let recently_failed = self
            .last_connect_failure
            .get()
            .is_some_and(|at| at.elapsed() < RECONNECT_INTERVAL);
        if recently_failed {
            return false;
        }
        self.connect();
        self.engine.borrow().is_some()
    }

    /// 带上当前中英模式，把虚拟键翻成 [`KeyEvent`]。
    fn key_event(&self, vk: u32) -> KeyEvent {
        keys::to_key_event(vk, self.english_mode.get())
    }

    /// 单击 Shift 切换中英模式：先把组着的内容原样落定，翻转状态，再更新系统的中 / 英指示器。
    fn toggle_english_mode(&self) {
        self.commit_pending();
        let english = !self.english_mode.get();
        self.english_mode.set(english);
        self.update_mode_indicator();
        log(if english {
            "切到英文模式"
        } else {
            "切到中文模式"
        });
    }

    /// 把系统任务栏的中 / 英指示器同步到当前模式：语言栏按钮换图标，再顺带写转换模式 compartment。
    fn update_mode_indicator(&self) {
        let english = self.english_mode.get();
        self.mode_state.set_english(english);
        if let Some(thread_mgr) = self.thread_mgr.borrow().as_ref() {
            super::mode::set_indicator(thread_mgr, self.client_id.get(), english);
        }
    }

    /// 在系统语言栏上登记中 / 英按钮（Win11 显示在品牌图标左边）。取不到管理器只记日志。
    fn add_lang_bar_item(&self) {
        let button = ModeButton::create(self.mode_state.clone());
        if let Some(thread_mgr) = self.thread_mgr.borrow().as_ref() {
            match thread_mgr.cast::<ITfLangBarItemMgr>() {
                // SAFETY: mgr 有效；button 是本 DLL 的语言栏项。
                Ok(mgr) => {
                    if let Err(error) = unsafe { mgr.AddItem(&button) } {
                        log(&format!("登记中英指示器失败: {error}"));
                    }
                }
                Err(error) => log(&format!("取语言栏管理器失败: {error}")),
            }
        }
        *self.mode_button.borrow_mut() = Some(button);
    }

    /// 反注册中 / 英按钮。
    fn remove_lang_bar_item(&self) {
        if let Some(button) = self.mode_button.borrow_mut().take()
            && let Some(thread_mgr) = self.thread_mgr.borrow().as_ref()
            && let Ok(mgr) = thread_mgr.cast::<ITfLangBarItemMgr>()
        {
            // SAFETY: mgr 有效；button 是之前登记的同一项。
            let _ = unsafe { mgr.RemoveItem(&button) };
        }
    }

    /// 这个键吃不吃。`OnTestKeyDown` 用，除了记 [`Self::shift_alone`] 无别的副作用，且与 [`Self::handle_key`] 一致。
    /// 与 Router 对齐：带 Ctrl/Alt/Win 一律放行（快捷键归应用），只有组句中的修饰键 + 数字送 Server 按配置判是不是
    /// 译词上屏 / 删候选（没配到的 Server 回 Passthrough）；字母在英文模式 / Caps 亮 / 小写（拼音）/ 组句中都吃，
    /// 只有中文模式下没在组句时按住 Shift 的大写字母直接归应用（临时打英文交给应用）；组句中功能键、方向键、可打印字符都吃。
    fn would_eat(&self, event: &KeyEvent) -> bool {
        let modifiers = event.modifiers;
        if modifiers.has_command_key() {
            return self.shared.composing() && keys::digit_key(event.virtual_key);
        }
        let vk = event.virtual_key;
        if keys::is_letter(vk) {
            return modifiers.caps
                || modifiers.english_mode
                || !modifiers.shift
                || self.shared.composing();
        }
        self.shared.composing()
            && (keys::is_edit(vk)
                || keys::is_nav(vk)
                || event.character.is_some_and(|c| !c.is_control()))
    }

    /// 不吃的键绝不碰组句（否则光标一移，组句会把拼音重插到别处）。要吃的转发 Server，按结果更新组句、候选窗、文档。
    fn handle_key(&self, pic: Ref<ITfContext>, event: KeyEvent) -> bool {
        if !self.would_eat(&event) {
            return false;
        }
        let vk = event.virtual_key;
        // 没连上 Server（没起 / 中途断了）：试着重连；还是不行就吃掉这个键别让它漏进应用，但没有引擎产出。
        if !self.ensure_connected() {
            return true;
        }
        if let Ok(context) = pic.ok() {
            self.shared.set_last_context(Some(context.clone()));
        }
        // Server 交互在这段借用里做完，放掉借用再走编辑会话。
        let update = {
            let mut guard = self.engine.borrow_mut();
            let Some(client) = guard.as_mut() else {
                return true;
            };
            // 组句被应用终止过：Server 里还留着那串拼音，先让它清掉（文本已在文档里，交出的丢弃）。
            let stale = self.shared.take_server_stale();
            let response = if stale {
                client.commit().and_then(|_| client.key(event))
            } else {
                client.key(event)
            };
            match response {
                Ok(response) => {
                    let preedit = preedit_string(&response.frame);
                    self.shared.set_composing(!response.frame.is_empty());
                    self.shared.update_candidates(&response.frame);
                    let consumed = matches!(response.outcome, KeyOutcome::Consumed);
                    let m = event.modifiers;
                    log(&format!(
                        "收键 vk={vk} ctrl={} alt={} shift={} caps={} en={} char={:?} candidates={} preedit={preedit:?} consumed={consumed}",
                        m.ctrl,
                        m.alt,
                        m.shift,
                        m.caps,
                        m.english_mode,
                        event.character,
                        response.frame.candidates.items.len()
                    ));
                    Some((response.commit, preedit, consumed))
                }
                Err(error) => {
                    log(&format!("转发按键失败，放行并断开，下一键重连: {error}"));
                    *guard = None;
                    self.last_connect_failure.set(None);
                    self.shared.end_composing();
                    None
                }
            }
        };
        match update {
            Some((commit, preedit, consumed)) => {
                self.update_document(pic, commit, preedit);
                consumed
            }
            None => false,
        }
    }

    /// 失焦 / 停用：让 Server 交出缓冲区，原样落进最近收键的文档并收掉组句。
    /// 组句已被应用终止的（拼音已是普通文本）只清 Server 不再插；没在组句就什么都不做。
    fn commit_pending(&self) {
        let stale = self.shared.take_server_stale();
        if !self.shared.composing() && !stale {
            return;
        }
        let text = {
            let mut guard = self.engine.borrow_mut();
            let Some(client) = guard.as_mut() else {
                self.shared.reset();
                return;
            };
            match client.commit() {
                Ok(text) => text,
                Err(error) => {
                    log(&format!("失焦上屏失败，断开，下一键重连: {error}"));
                    *guard = None;
                    self.last_connect_failure.set(None);
                    self.shared.end_composing();
                    return;
                }
            }
        };
        if stale {
            return;
        }
        self.shared.end_composing();
        let Some(context) = self.shared.last_context() else {
            log(&format!("失焦上屏没有上下文，丢弃: {text:?}"));
            self.shared.reset();
            return;
        };
        log(&format!("失焦上屏: {text:?}"));
        let requested = super::edit_session::request_update(
            &context,
            self.client_id.get(),
            self.shared.clone(),
            text.filter(|t| !t.is_empty()),
            String::new(),
        );
        if let Err(error) = requested {
            log(&format!("失焦上屏的编辑会话没被受理: {error}"));
            self.shared.reset();
        }
    }

    /// 经异步编辑会话把上屏文本 + 组句拼音行写进文档。
    fn update_document(&self, pic: Ref<ITfContext>, commit: Option<String>, preedit: String) {
        // 什么都不用改就不跑编辑会话。最后一项：退到最后一个字母时帧已空，但组句句柄还在，得跑一次把它收掉。
        if commit.is_none()
            && preedit.is_empty()
            && !self.shared.composing()
            && !self.shared.has_composition()
        {
            return;
        }
        let Ok(context) = pic.ok() else {
            log(&format!(
                "无上下文，丢弃更新: commit={commit:?} preedit={preedit:?}"
            ));
            return;
        };
        if let Err(error) = super::edit_session::request_update(
            context,
            self.client_id.get(),
            self.shared.clone(),
            commit,
            preedit,
        ) {
            log(&format!("请求组句更新失败: {error}"));
        }
    }
}
