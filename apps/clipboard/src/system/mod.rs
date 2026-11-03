//! 和 Windows 打交道的部分：后台线程收剪贴板变化与全局快捷键，读写系统剪贴板，粘贴回原窗口。

mod clipboard;
mod listener;
mod neon;
mod paste;
mod source;
mod tray;

use qingjian_clipboard::{Capture, ClipSettings, WorkArea};
use qingjian_clipboard_sync::SyncEvent;

pub use clipboard::{Payload, write_payload};
pub use listener::{Watch, spawn_listener};
pub use neon::Neon;
pub use paste::{copy, paste, refocus};
pub use source::load_icon;

/// 占住全机（本用户）唯一的名字；已经开着一个时为 `false`。句柄不关，进程退出时系统收回。
pub fn claim_single_instance() -> bool {
    use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::core::w;

    // SAFETY: 名字是编译期的 UTF-16 常量，没有安全属性。
    let created = unsafe { CreateMutexW(None, false, w!("Local\\QingjianClipboard")) };
    created.is_ok() && unsafe { GetLastError() } != ERROR_ALREADY_EXISTS
}

/// 这个键现在按着没有。
pub fn key_down(key: u32) -> bool {
    // SAFETY: 只读系统的按键状态。
    unsafe { windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(key as i32) < 0 }
}

/// 后台线程送给面板的事。
pub enum Signal {
    /// 复制了内容（已排除密码管理器标记为敏感的）。
    Copied(Capture),

    /// 配置改了。
    Settings(ClipSettings),

    /// 同步服务的消息。
    Sync(SyncEvent),

    /// 按了呼出快捷键。
    Hotkey {
        /// 当时的前台窗口（`HWND` 的值）。
        target: isize,

        /// 光标所在显示器的工作区。
        area: WorkArea,

        /// 那台显示器的点 → 像素倍数。
        scale: f32,

        /// 快捷键的主键（虚拟键码）：这时还按着，松开之前的按键不算打字。从托盘打开时没有。
        held_key: Option<u32>,
    },
}
