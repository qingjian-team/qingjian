//! 文本服务对象：实现 [`ITfTextInputProcessor`]（激活 / 停用）与 [`ITfKeyEventSink`]（收键）。
//!
//! 激活时把自己挂到线程的击键管理器上收键，并连独立 Server 进程（引擎在那边）。收到键就转成
//! [`KeyEvent`] 转发给 Server，按结果：决定吃不吃键、把要上屏的文本落定、把组句拼音行经 TSF 组句
//! （[`ITfComposition`]，见 [`super::composition`]）显示在文档光标处。候选窗口是下一步。

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use windows::Win32::Foundation::{FALSE, LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, VIRTUAL_KEY, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::TextServices::{
    ITfContext, ITfKeyEventSink, ITfKeyEventSink_Impl, ITfKeystrokeMgr, ITfTextInputProcessor,
    ITfTextInputProcessor_Impl, ITfThreadMgr,
};
use windows::core::{BOOL, GUID, IUnknownImpl, Interface, Ref, Result, implement};

use qingjian_platform::protocol::{KeyEvent, KeyModifiers, KeyOutcome, SessionId};

use super::composition::{Shared, preedit_string};
use crate::client::EngineClient;
use crate::client::pipe::{PipeStream, connect_default};

/// 一个 TSF 文本服务实例（TSF 每个线程一个）。用 `#[implement]` 成为实现两个接口的 COM 对象。
#[implement(ITfTextInputProcessor, ITfKeyEventSink)]
pub struct TextService {
    /// 激活时拿到的线程管理器，停用时用它反注册击键 sink。
    thread_mgr: RefCell<Option<ITfThreadMgr>>,

    /// TSF 分配的 client id，反注册击键 sink 要用。
    client_id: Cell<u32>,

    /// 连 Server 的会话客户端（引擎层）。连不上时为 `None`，键照样放行。
    engine: RefCell<Option<EngineClient<PipeStream>>>,

    /// 跨按键存活的组句状态（活动组句 + 组句影子标志），与编辑会话 / 组句 sink 共享。
    shared: Rc<Shared>,
}

impl TextService {
    #[allow(clippy::new_without_default)] // new 有 lock_module 副作用，不宜 Default
    pub fn new() -> Self {
        super::lock_module();
        Self {
            thread_mgr: RefCell::new(None),
            client_id: Cell::new(0),
            engine: RefCell::new(None),
            shared: Shared::new(),
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
        super::log::log("Activate: 开始激活");
        let thread_mgr = ptim.ok()?.clone();

        // 收键：把自己作为 ITfKeyEventSink 挂到击键管理器上。
        let keystroke: ITfKeystrokeMgr = thread_mgr.cast()?;
        let sink: ITfKeyEventSink = self.to_interface();
        // SAFETY: keystroke 有效；sink 是本对象的接口。
        unsafe { keystroke.AdviseKeyEventSink(tid, &sink, true)? };

        // 连 Server（引擎在那边）。连不上不致命：记日志，键照样放行，等 Server 起来。
        match connect_default() {
            Ok(stream) => match EngineClient::open(stream, SessionId(tid as u64)) {
                Ok(client) => *self.engine.borrow_mut() = Some(client),
                Err(error) => super::log::log(&format!("开会话失败: {error}")),
            },
            Err(error) => super::log::log(&format!(
                "连 Server 管道失败（qingjian-server 没起？）: {error}"
            )),
        }

        // 建候选窗口（不抢焦点的浮动弹窗）。失败不致命：降级为无候选 UI。
        match super::candidates::CandidateWindow::new() {
            Ok(window) => self.shared.set_window(Some(window)),
            Err(error) => super::log::log(&format!("建候选窗口失败: {error}")),
        }

        *self.thread_mgr.borrow_mut() = Some(thread_mgr);
        self.client_id.set(tid);
        super::log::log(&format!("青简 TSF 已激活 tid={tid}"));
        Ok(())
    }

    fn Deactivate(&self) -> Result<()> {
        if let Some(thread_mgr) = self.thread_mgr.borrow_mut().take()
            && let Ok(keystroke) = thread_mgr.cast::<ITfKeystrokeMgr>()
        {
            // SAFETY: keystroke 有效。
            let _ = unsafe { keystroke.UnadviseKeyEventSink(self.client_id.get()) };
        }
        if let Some(client) = self.engine.borrow_mut().take() {
            let _ = client.close();
        }
        // 丢掉本地组句句柄、收起候选窗口（上下文即将失效，不再走编辑会话收尾），再销毁候选窗口。
        self.shared.reset();
        self.shared.set_window(None);
        super::log::log("青简 TSF 已停用");
        Ok(())
    }
}

impl ITfKeyEventSink_Impl for TextService_Impl {
    fn OnSetFocus(&self, _fforeground: BOOL) -> Result<()> {
        Ok(())
    }

    fn OnTestKeyDown(&self, _pic: Ref<ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        Ok(self
            .would_eat(&to_key_event(wparam.0 as u32, lparam))
            .into())
    }

    fn OnKeyDown(&self, pic: Ref<ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        Ok(self
            .handle_key(pic, to_key_event(wparam.0 as u32, lparam))
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
    /// 这个键我们会不会吃（`OnTestKeyDown` 用，必须无副作用，且要与 `handle_key` 的实际处理一致）。
    /// 规则与 Server 的 Router 对齐：**带 Ctrl/Alt/Win 的组合一律放行**（交给应用做快捷键，如 Ctrl+Z）；
    /// 字母总吃（起 / 续组句）；组句中时功能键（空格 / 回车 / 退格 / Esc / 数字）、方向 / 翻页键、以及任何
    /// 可打印字符（半角标点进英文直输段）都吃。
    fn would_eat(&self, event: &KeyEvent) -> bool {
        let modifiers = event.modifiers;
        if modifiers.ctrl || modifiers.alt || modifiers.win {
            return false;
        }
        let vk = event.virtual_key;
        if is_letter_vk(vk) {
            return true;
        }
        self.shared.composing()
            && (is_edit_vk(vk) || is_nav_vk(vk) || event.character.is_some_and(|c| !c.is_control()))
    }

    /// 真收键：`would_eat` 不吃的（带 Ctrl/Alt/Win、或非组句下的非字母键）直接放行、绝不碰组句（否则
    /// 光标一移、我们的组句又把拼音重插到别处，拼音会撒一屏）。要吃的才转发给 Server，按结果更新组句影子
    /// 标志、灌候选窗口、把上屏文本 + 拼音行经编辑会话写文档、记日志，返回是否吃键。连接坏了丢弃并放行。
    fn handle_key(&self, pic: Ref<ITfContext>, event: KeyEvent) -> bool {
        if !self.would_eat(&event) {
            return false;
        }
        let vk = event.virtual_key;
        // Server 交互只在这段借用里做完；拿到「上屏文本 + 拼音行 + 是否吃键」后放掉借用，再走编辑会话
        // （编辑会话回调不碰 engine，但先放掉借用更清爽，也避免万一的重入）。
        let update = {
            let mut guard = self.engine.borrow_mut();
            let Some(client) = guard.as_mut() else {
                // 没连上 Server：would_eat 已判要吃（多为字母），吃掉别让它漏进应用，但没有引擎产出。
                return true;
            };
            match client.key(event) {
                Ok(response) => {
                    let preedit = preedit_string(&response.frame);
                    self.shared.set_composing(!response.frame.is_empty());
                    // 把这帧候选灌进候选窗口（定位与显示留给编辑会话，那时才有光标位置）。
                    self.shared.update_candidates(&response.frame);
                    let consumed = matches!(response.outcome, KeyOutcome::Consumed);
                    super::log::log(&format!(
                        "收键 vk={vk} candidates={} preedit={preedit:?} consumed={consumed}",
                        response.frame.candidates.items.len()
                    ));
                    Some((response.commit, preedit, consumed))
                }
                Err(error) => {
                    super::log::log(&format!("转发按键失败，放行并断开: {error}"));
                    *guard = None;
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

    /// 经 TSF 异步编辑会话把上屏文本 + 组句拼音行写进当前文档。没有上下文或会话被拒时记日志、不致命。
    fn update_document(&self, pic: Ref<ITfContext>, commit: Option<String>, preedit: String) {
        // 这次什么都不用改（不上屏、无新组句、也没留着的活动组句要收）就别费编辑会话了。
        // 注意最后那个 has_composition：退到最后一个拼音字母时 frame 变空、composing 已置 false，但组句句柄
        // 还在——必须跑编辑会话把它 EndComposition 收掉，否则残留、要按两次退格才消（曾经的 bug）。
        if commit.is_none()
            && preedit.is_empty()
            && !self.shared.composing()
            && !self.shared.has_composition()
        {
            return;
        }
        let Ok(context) = pic.ok() else {
            super::log::log(&format!(
                "无上下文，丢弃更新: commit={commit:?} preedit={preedit:?}"
            ));
            return;
        };
        match super::edit_session::request_update(
            context,
            self.client_id.get(),
            self.shared.clone(),
            commit,
            preedit,
        ) {
            Ok(()) => super::log::log("已请求组句更新"),
            Err(error) => super::log::log(&format!("请求组句更新失败: {error}")),
        }
    }
}

/// A–Z（`0x41..=0x5A`）。
fn is_letter_vk(vk: u32) -> bool {
    (0x41..=0x5A).contains(&vk)
}

/// 组句中要吃的功能键：退格 / 回车 / Esc / 空格 / 数字 0–9。
fn is_edit_vk(vk: u32) -> bool {
    matches!(vk, 0x08 | 0x0D | 0x1B | 0x20) || (0x30..=0x39).contains(&vk)
}

/// 组句中要吃的方向 / 翻页键：PageUp/Down、End、Home、← ↑ → ↓（`0x21..=0x28`）。移候选 / 翻页 / 移光标。
fn is_nav_vk(vk: u32) -> bool {
    (0x21..=0x28).contains(&vk)
}

/// 把虚拟键码转成协议的 [`KeyEvent`]：采当前修饰键，解析出对应字符（字母小写、数字、空格、半角标点），
/// Router 靠 `character` 分派（半角标点进英文直输段）。用当前键盘状态（`GetKeyState`），无副作用。
fn to_key_event(vk: u32, _lparam: LPARAM) -> KeyEvent {
    let modifiers = current_modifiers();
    let character = resolve_char(vk, modifiers.shift);
    KeyEvent::new(vk, character, modifiers)
}

/// 采当前 Ctrl / Shift / Alt / Win 状态（`GetKeyState` 高位）。
fn current_modifiers() -> KeyModifiers {
    KeyModifiers {
        ctrl: key_down(VK_CONTROL),
        shift: key_down(VK_SHIFT),
        alt: key_down(VK_MENU),
        win: key_down(VK_LWIN) || key_down(VK_RWIN),
    }
}

/// 某个虚拟键此刻是否按下（`GetKeyState` 返回值的高位为 1 表示按下）。
fn key_down(vk: VIRTUAL_KEY) -> bool {
    // SAFETY: GetKeyState 只读当前线程消息队列里的键状态。
    (unsafe { GetKeyState(vk.0 as i32) } as u16 & 0x8000) != 0
}

/// 虚拟键 + Shift → 字符（US 布局）：字母恒小写，数字 / 半角标点按 Shift 取对应符号。功能键返回 `None`。
fn resolve_char(vk: u32, shift: bool) -> Option<char> {
    // 字母：拼音用小写（英文直输段的大小写下一步再说）。
    if (0x41..=0x5A).contains(&vk) {
        return Some((b'a' + (vk - 0x41) as u8) as char);
    }
    // 数字行：Shift 取上排符号，否则数字（数字在组句里选候选，符号进英文段）。
    if (0x30..=0x39).contains(&vk) {
        let digit = (vk - 0x30) as usize;
        return Some(if shift {
            b")!@#$%^&*("[digit] as char
        } else {
            (b'0' + digit as u8) as char
        });
    }
    if vk == 0x20 {
        return Some(' ');
    }
    // OEM 标点键（US 布局）：(不按 Shift, 按 Shift)。
    let (plain, shifted) = match vk {
        0xBA => (';', ':'),
        0xBB => ('=', '+'),
        0xBC => (',', '<'),
        0xBD => ('-', '_'),
        0xBE => ('.', '>'),
        0xBF => ('/', '?'),
        0xC0 => ('`', '~'),
        0xDB => ('[', '{'),
        0xDC => ('\\', '|'),
        0xDD => (']', '}'),
        0xDE => ('\'', '"'),
        _ => return None,
    };
    Some(if shift { shifted } else { plain })
}
