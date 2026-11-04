//! 和 macOS 打交道的部分：后台线程轮询剪贴板，主线程上收全局快捷键、挂菜单栏图标，读写系统剪贴板，粘贴回原来的程序，霓虹灯的窗口。

mod autostart;
mod ffi;
mod hotkey;
mod keycode;
mod listener;
mod main_queue;
mod menu_state;
mod neon_strips;
mod paste;
mod pasteboard;
mod screen;
mod source;
mod status_item;

use std::fs::File;
use std::path::PathBuf;

pub use listener::{Watch, spawn_listener};
pub(crate) use neon_strips::{Strips, animations_on, should_hide};
pub use paste::{copy, paste, refocus};
pub use pasteboard::write_payload;
pub use source::load_icon;

/// 锁住本用户唯一的锁文件；已经开着一个时为 `false`。文件不关，进程退出时系统收回锁。
pub fn claim_single_instance() -> bool {
    let Some(home) = std::env::var_os("HOME") else {
        return true;
    };
    let directory = PathBuf::from(home).join("Library/Application Support/Qingjian/clipboard");
    if let Err(error) = std::fs::create_dir_all(&directory) {
        tracing::warn!(%error, "cannot create the clipboard directory");
        return true;
    }
    let file = match File::create(directory.join("instance.lock")) {
        Ok(file) => file,
        Err(error) => {
            tracing::warn!(%error, "cannot open the instance lock");
            return true;
        }
    };
    if file.try_lock().is_err() {
        return false;
    }
    std::mem::forget(file);
    true
}

/// 给配对的设备看的电脑名（「系统设置 → 通用 → 共享」里那个）；取不到为空。
pub fn computer_name() -> String {
    // SAFETY: 不带 store 时读系统的；返回的 CFString 与 NSString 免费桥接，交给 Retained 释放。
    let name =
        unsafe { ffi::SCDynamicStoreCopyComputerName(std::ptr::null(), std::ptr::null_mut()) };
    if name.is_null() {
        return String::new();
    }
    // SAFETY: 见上；Copy 规则下引用归我们。
    unsafe { objc2::rc::Retained::from_raw(name.cast_mut().cast::<objc2_foundation::NSString>()) }
        .map(|name| name.to_string())
        .unwrap_or_default()
}

/// 这个键（macOS 键码）现在按着没有。
pub fn key_down(key: u32) -> bool {
    let Ok(key) = u16::try_from(key) else {
        return false;
    };
    // SAFETY: 只读系统的按键状态。
    unsafe { ffi::CGEventSourceKeyState(ffi::COMBINED_SESSION_STATE, key) }
}
