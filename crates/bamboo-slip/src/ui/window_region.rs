//! 自绘标题栏里的窗口区域标记：Windows 宿主按它给系统回报 `WM_NCHITTEST`，
//! 让拖动、双击最大化、右键系统菜单与 Windows 11 最大化按钮的贴靠布局都按原生方式工作。

/// 元素在自绘标题栏里充当的窗口区域。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowRegion {
    Drag,

    Minimize,

    Maximize,

    Close,
}

impl WindowRegion {
    pub(super) fn is_button(self) -> bool {
        !matches!(self, Self::Drag)
    }
}

/// 单个可交互元素在客户区里的矩形与标题栏区域标记，按画序收给 Windows `WM_NCHITTEST` 查。
#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Clone, Copy)]
pub(crate) struct WindowHit {
    pub(crate) rect: (f32, f32, f32, f32),
    pub(crate) region: Option<WindowRegion>,
}
