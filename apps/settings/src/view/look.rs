//! 这次画哪套外观：macOS 照「系统设置」、Windows 照 Windows 11「设置」，其余平台用青简自己的竹青外观。
//! 开发时设环境变量 `QINGJIAN_SETTINGS_LOOK=mac|fluent|plain` 换一套看（在 Mac 上截 Windows 那套的图）。

use std::sync::OnceLock;

use bamboo_slip::Look;

pub fn current() -> Look {
    static LOOK: OnceLock<Look> = OnceLock::new();
    *LOOK.get_or_init(
        || match std::env::var("QINGJIAN_SETTINGS_LOOK").as_deref() {
            Ok("mac") => Look::Mac,
            Ok("fluent") => Look::Fluent,
            Ok("plain") => Look::Plain,
            _ if cfg!(target_os = "macos") => Look::Mac,
            _ if cfg!(windows) => Look::Fluent,
            _ => Look::Plain,
        },
    )
}

/// 窗口初始的内容区大小（点）：Windows 照「设置」开得宽一些，左侧导航约占三成。
pub fn window_size(look: Look) -> (f32, f32) {
    match look {
        Look::Fluent => (1040.0, 680.0),
        Look::Mac | Look::Plain => (900.0, 680.0),
    }
}
