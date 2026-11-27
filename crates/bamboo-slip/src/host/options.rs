//! 开窗口的参数。

use crate::backdrop::Backdrop;

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

    /// 窗口背后的系统材质：根节点的底色要半透明才透得出来。系统不支持（[`Backdrop::available`]）时不要设。
    pub backdrop: Option<Backdrop>,

    /// Windows 上去掉原生标题栏、把客户区铺到窗口最顶（保留边框、阴影、圆角与贴靠布局）；
    /// 应用用 [`crate::WindowRegion`] 标出拖动区与右上角三个按钮。其他平台忽略。
    pub custom_titlebar: bool,
}

impl WindowOptions {
    pub fn new(title: impl Into<String>, size: (f32, f32)) -> Self {
        Self {
            title: title.into(),
            size,
            min_size: (size.0 * 0.6, size.1 * 0.5),
            locale: "zh-CN".to_owned(),
            panel: false,
            backdrop: None,
            custom_titlebar: false,
        }
    }

    /// 见 [`Self::backdrop`]。
    pub fn backdrop(self, backdrop: Option<Backdrop>) -> Self {
        Self { backdrop, ..self }
    }

    /// 见 [`Self::custom_titlebar`]。
    pub fn custom_titlebar(self, custom_titlebar: bool) -> Self {
        Self {
            custom_titlebar,
            ..self
        }
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
