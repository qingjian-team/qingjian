//! 窗口背后的系统材质：窗口里没盖满（半透明）的地方透出系统画的模糊底。由 [`crate::WindowOptions::backdrop`] 要，
//! 使用方先用 [`Backdrop::available`] 判断，拿不到时底色画成不透明的。

/// 哪种材质。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backdrop {
    /// 主窗口的底：Windows 11 的 Mica，macOS 的侧栏毛玻璃（`NSVisualEffectView`）。
    Window,

    /// 临时浮起的面板：Windows 11 的亚克力；macOS 不支持。
    Transient,
}

impl Backdrop {
    /// 系统支持这种材质，用户也没关「透明效果」（Windows）/「减少透明度」（macOS）。
    /// Windows 第一个支持的版本是 Windows 11 22H2（22621）。
    pub fn available(self) -> bool {
        #[cfg(target_os = "windows")]
        {
            let _ = self;
            crate::appearance::read_build() >= Some(22621)
                && crate::appearance::read_transparency() != Some(0)
        }
        #[cfg(target_os = "macos")]
        {
            use objc2_app_kit::NSWorkspace;

            self == Self::Window
                && !NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceTransparency()
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            let _ = self;
            false
        }
    }
}
