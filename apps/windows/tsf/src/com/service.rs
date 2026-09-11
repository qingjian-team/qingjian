//! 文本服务对象：[`ITfTextInputProcessor`]（激活 / 停用）与 [`ITfKeyEventSink`]（收键）。

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{FALSE, LPARAM, WPARAM};
use windows::Win32::UI::TextServices::{
    ITfContext, ITfKeyEventSink, ITfKeyEventSink_Impl, ITfKeystrokeMgr, ITfTextInputProcessor,
    ITfTextInputProcessor_Impl, ITfThreadMgr,
};
use windows::core::{BOOL, GUID, IUnknownImpl, Interface, Ref, Result, implement};

use qingjian_platform::protocol::{KeyEvent, KeyOutcome, SessionId};

use super::candidates::CandidateWindow;
use super::composition::{Shared, preedit_string};
use super::keys;
use super::log::log;
use super::poll::PollTimer;
use crate::client::EngineClient;
use crate::client::pipe::{PipeStream, connect_default};

/// 连 Server 的会话客户端，与云联想轮询定时器共享（STA 单线程，不并发）。连不上时为 `None`，键照样放行。
pub(crate) type SharedClient = Rc<RefCell<Option<EngineClient<PipeStream>>>>;

/// 连不上 Server 后隔多久再试。每次尝试连接都在应用的 UI 线程上，不能每键都试。
const RECONNECT_INTERVAL: Duration = Duration::from_secs(2);

/// 一个 TSF 文本服务实例（每线程一个）。
#[implement(ITfTextInputProcessor, ITfKeyEventSink)]
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
        log(&format!("青简 TSF 已激活 tid={tid}"));
        Ok(())
    }

    fn Deactivate(&self) -> Result<()> {
        // 先停定时器，之后不再有回调碰 engine / shared。
        self.poll_timer.borrow_mut().take();
        if let Some(thread_mgr) = self.thread_mgr.borrow_mut().take()
            && let Ok(keystroke) = thread_mgr.cast::<ITfKeystrokeMgr>()
        {
            // SAFETY: keystroke 有效。
            let _ = unsafe { keystroke.UnadviseKeyEventSink(self.client_id.get()) };
        }
        if let Some(client) = self.engine.borrow_mut().take() {
            let _ = client.close();
        }
        // 上下文即将失效，不再走编辑会话收尾：直接丢组句句柄、销毁候选窗口。
        self.shared.reset();
        self.shared.set_window(None);
        log("青简 TSF 已停用");
        Ok(())
    }
}

impl ITfKeyEventSink_Impl for TextService_Impl {
    /// 获得焦点时顺手补一次连接：Server 起晚了、或中途重启过，切回来就能用，不必切走再切回输入法。
    fn OnSetFocus(&self, fforeground: BOOL) -> Result<()> {
        if fforeground.as_bool() {
            self.ensure_connected();
        }
        Ok(())
    }

    fn OnTestKeyDown(
        &self,
        _pic: Ref<ITfContext>,
        wparam: WPARAM,
        _lparam: LPARAM,
    ) -> Result<BOOL> {
        Ok(self.would_eat(&keys::to_key_event(wparam.0 as u32)).into())
    }

    fn OnKeyDown(&self, pic: Ref<ITfContext>, wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        Ok(self
            .handle_key(pic, keys::to_key_event(wparam.0 as u32))
            .into())
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

impl TextService_Impl {
    /// 连 Server 并开会话（会话 id 用 TSF 的 client id）。失败记时间，供 [`Self::ensure_connected`] 退避。
    fn connect(&self) {
        let session = SessionId(self.client_id.get() as u64);
        let connected = connect_default()
            .map_err(|e| e.to_string())
            .and_then(|stream| EngineClient::open(stream, session).map_err(|e| e.to_string()));
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

    /// 这个键吃不吃。`OnTestKeyDown` 用，必须无副作用，且与 [`Self::handle_key`] 一致。
    /// 与 Router 对齐：带 Ctrl/Alt/Win 一律放行（快捷键归应用）；字母总吃；组句中功能键、方向键、可打印字符都吃。
    fn would_eat(&self, event: &KeyEvent) -> bool {
        let modifiers = event.modifiers;
        if modifiers.ctrl || modifiers.alt || modifiers.win {
            return false;
        }
        let vk = event.virtual_key;
        if keys::is_letter(vk) {
            return true;
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
        // Server 交互在这段借用里做完，放掉借用再走编辑会话。
        let update = {
            let mut guard = self.engine.borrow_mut();
            let Some(client) = guard.as_mut() else {
                return true;
            };
            match client.key(event) {
                Ok(response) => {
                    let preedit = preedit_string(&response.frame);
                    self.shared.set_composing(!response.frame.is_empty());
                    self.shared.update_candidates(&response.frame);
                    let consumed = matches!(response.outcome, KeyOutcome::Consumed);
                    log(&format!(
                        "收键 vk={vk} candidates={} preedit={preedit:?} consumed={consumed}",
                        response.frame.candidates.items.len()
                    ));
                    Some((response.commit, preedit, consumed))
                }
                Err(error) => {
                    log(&format!("转发按键失败，放行并断开，下一键重连: {error}"));
                    *guard = None;
                    self.last_connect_failure.set(None);
                    self.shared.disconnected();
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
