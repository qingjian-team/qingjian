//! 和 Windows 打交道的部分：消息窗口线程收剪贴板变化与全局快捷键，读写系统剪贴板，粘贴回原窗口，托盘与霓虹灯的窗口。

mod clipboard;
mod listener;
mod neon_strip;
mod neon_strips;
mod paste;
mod source;
mod tray;

pub use clipboard::write_payload;
pub use listener::{Watch, spawn_listener};
pub(crate) use neon_strips::{Strips, animations_on, should_hide};
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

/// 给配对的设备看的电脑名。
pub fn computer_name() -> String {
    std::env::var("COMPUTERNAME").unwrap_or_default()
}

/// 这个键现在按着没有。
pub fn key_down(key: u32) -> bool {
    // SAFETY: 只读系统的按键状态。
    unsafe { windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(key as i32) < 0 }
}
