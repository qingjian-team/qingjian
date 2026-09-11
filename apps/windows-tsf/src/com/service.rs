//! 文本服务对象：实现 [`ITfTextInputProcessor`]（激活 / 停用）与 [`ITfKeyEventSink`]（收键）。
//!
//! 激活时把自己挂到线程的击键管理器上收键，并连独立 Server 进程（引擎在那边）。收到键就转成
//! [`KeyEvent`] 转发给 Server、按结果决定吃不吃键。**这一步先不画候选、不上屏**：上屏 / preedit 要走
//! `ITfContext` 编辑会话，是下一步；现在把 Server 的结果记进日志，验证「真实应用 → TSF → 管道 → Server
//! → 引擎」整条链路通了。

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use windows::Win32::Foundation::{FALSE, LPARAM, WPARAM};
use windows::Win32::UI::TextServices::{
    ITfContext, ITfKeyEventSink, ITfKeyEventSink_Impl, ITfKeystrokeMgr, ITfTextInputProcessor,
    ITfTextInputProcessor_Impl, ITfThreadMgr,
};
use windows::core::{BOOL, GUID, IUnknownImpl, Interface, Ref, Result, implement};

use qingjian_platform::protocol::{KeyEvent, KeyModifiers, KeyOutcome, SessionId};

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

    /// 组句影子标志：Server 每次回的 frame 空不空，决定 `OnTestKeyDown` 要不要吃功能键。
    composing: Cell<bool>,
}

impl TextService {
    #[allow(clippy::new_without_default)] // new 有 lock_module 副作用，不宜 Default
    pub fn new() -> Self {
        super::lock_module();
        Self {
            thread_mgr: RefCell::new(None),
            client_id: Cell::new(0),
            engine: RefCell::new(None),
            composing: Cell::new(false),
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
        init_logging();
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
                Err(error) => tracing::warn!(%error, "开会话失败"),
            },
            Err(error) => tracing::warn!(%error, "连 Server 管道失败（qingjian-server 没起？）"),
        }

        *self.thread_mgr.borrow_mut() = Some(thread_mgr);
        self.client_id.set(tid);
        tracing::info!(tid, "青简 TSF 已激活");
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
        self.composing.set(false);
        tracing::info!("青简 TSF 已停用");
        Ok(())
    }
}

impl ITfKeyEventSink_Impl for TextService_Impl {
    fn OnSetFocus(&self, _fforeground: BOOL) -> Result<()> {
        Ok(())
    }

    fn OnTestKeyDown(
        &self,
        _pic: Ref<ITfContext>,
        wparam: WPARAM,
        _lparam: LPARAM,
    ) -> Result<BOOL> {
        Ok(self.would_eat(wparam.0 as u32).into())
    }

    fn OnKeyDown(&self, _pic: Ref<ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        Ok(self.handle_key(wparam.0 as u32, lparam).into())
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
    /// 这个虚拟键我们会不会吃（`OnTestKeyDown` 用，必须无副作用）。规则与 Server 的 Router 对齐：
    /// 字母总吃；组句中时空格 / 回车 / 退格 / Esc / 数字也吃。
    fn would_eat(&self, vk: u32) -> bool {
        is_letter_vk(vk) || (self.composing.get() && is_edit_vk(vk))
    }

    /// 真收键：转发给 Server，按结果更新组句影子标志、记日志，返回是否吃键。连接坏了就丢弃并放行。
    fn handle_key(&self, vk: u32, lparam: LPARAM) -> bool {
        let mut guard = self.engine.borrow_mut();
        let Some(client) = guard.as_mut() else {
            // 没连上 Server：按同样规则决定吃不吃，但没有引擎产出。
            return self.would_eat(vk);
        };
        match client.key(to_key_event(vk, lparam)) {
            Ok(response) => {
                self.composing.set(!response.frame.is_empty());
                if let Some(text) = &response.commit {
                    // TODO(下一步)：经 ITfContext 编辑会话把 text 插进文档。现在只记日志。
                    tracing::info!(commit = %text, "待上屏");
                }
                tracing::debug!(
                    vk,
                    candidates = response.frame.candidates.items.len(),
                    consumed = matches!(response.outcome, KeyOutcome::Consumed),
                    "收键"
                );
                matches!(response.outcome, KeyOutcome::Consumed)
            }
            Err(error) => {
                tracing::warn!(%error, "转发按键失败，放行并断开");
                *guard = None;
                false
            }
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

/// 把虚拟键码转成协议的 [`KeyEvent`]。字母 / 数字 / 空格附上对应字符（Router 靠 `character` 分派），
/// 其余只带虚拟键码。修饰键先不采（下一步用 `GetKeyboardState` 补 Ctrl/Shift 等）。
fn to_key_event(vk: u32, _lparam: LPARAM) -> KeyEvent {
    let character = if (0x41..=0x5A).contains(&vk) {
        Some((b'a' + (vk - 0x41) as u8) as char)
    } else if (0x30..=0x39).contains(&vk) {
        Some((b'0' + (vk - 0x30) as u8) as char)
    } else if vk == 0x20 {
        Some(' ')
    } else {
        None
    };
    KeyEvent::new(vk, character, KeyModifiers::default())
}

/// 只初始化一次的文件日志（`%LOCALAPPDATA%\Qingjian\tsf.log`）。DLL 没有控制台，日志是这一步唯一的
/// 观测手段。失败静默（没日志也不能拖垮宿主进程）。
fn init_logging() {
    static LOGGING: OnceLock<()> = OnceLock::new();
    LOGGING.get_or_init(|| {
        let Some(dir) = log_dir() else { return };
        let _ = std::fs::create_dir_all(&dir);
        let Ok(file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("tsf.log"))
        else {
            return;
        };
        let _ = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_max_level(tracing::Level::DEBUG)
            .with_writer(Mutex::new(file))
            .try_init();
    });
}

/// 日志目录：`%LOCALAPPDATA%\Qingjian`。
fn log_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|base| PathBuf::from(base).join("Qingjian"))
}
