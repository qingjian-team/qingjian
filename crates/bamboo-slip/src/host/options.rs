//! 开窗口的参数。

#[derive(Debug, Clone)]
pub struct WindowOptions {
    pub title: String,

    /// 初始与最小的内容区大小（点）。
    pub size: (f32, f32),

    pub min_size: (f32, f32),

    /// 中日字形回退按它挑字体，见 [`crate::FontLibrary::system`]。
    pub locale: String,

    /// 面板：开出来先藏着、无边框、总在最前、不进任务栏，由 [`crate::App::take_window_command`] 摆放与显隐。
    pub panel: bool,

    /// 面板背后用系统的亚克力材质（Windows 11）：根节点的底色要半透明才透得出来。
    pub acrylic: bool,
}

impl WindowOptions {
    pub fn new(title: impl Into<String>, size: (f32, f32)) -> Self {
        Self {
            title: title.into(),
            size,
            min_size: (size.0 * 0.6, size.1 * 0.5),
            locale: "zh-CN".to_owned(),
            panel: false,
            acrylic: false,
        }
    }

    /// 见 [`Self::acrylic`]；系统支持与否由 [`crate::Accent`] 旁的 [`crate::system_acrylic`] 判断。
    pub fn acrylic(self, acrylic: bool) -> Self {
        Self { acrylic, ..self }
    }

    /// 改成面板窗口，见 [`Self::panel`]。
    pub fn panel(self) -> Self {
        Self {
            panel: true,
            min_size: (1.0, 1.0),
            ..self
        }
    }
}
