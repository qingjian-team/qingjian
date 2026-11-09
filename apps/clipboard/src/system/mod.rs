//! 和系统打交道的部分：后台收剪贴板变化与全局快捷键，读写系统剪贴板，粘贴回原来的程序。各平台一个子模块，对外的函数同名。

mod neon;
mod payload;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

/// 当前平台的实现，平台无关的模块（霓虹灯）从这里取窗口。
#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(windows)]
use windows as platform;

use qingjian_clipboard::{Capture, ClipSettings, WorkArea};
use qingjian_clipboard_sync::SyncEvent;

#[cfg(target_os = "macos")]
pub use macos::{
    Watch, claim_single_instance, computer_name, copy, key_down, load_icon, paste, refocus,
    spawn_listener, write_payload,
};
pub use neon::Neon;
pub use payload::Payload;
#[cfg(windows)]
pub use windows::{
    Watch, claim_single_instance, computer_name, copy, key_down, load_icon, paste, refocus,
    spawn_listener, write_payload,
};

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
        /// 粘贴回去的目标：Windows 是当时的前台窗口（`HWND` 的值），macOS 是前台程序的进程号。
        target: isize,

        /// 光标所在显示器的工作区。
        area: WorkArea,

        /// 那台显示器的点 → 像素倍数。
        scale: f32,

        /// 快捷键的主键（Windows 的虚拟键码、macOS 的键码）：这时还按着，松开之前的按键不算打字。从托盘打开时没有。
        held_key: Option<u32>,
    },
}
