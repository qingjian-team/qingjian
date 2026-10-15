//! 开窗口的参数。

#[derive(Debug, Clone)]
pub struct WindowOptions {
    pub title: String,

    /// 初始与最小的内容区大小（点）。
    pub size: (f32, f32),

    pub min_size: (f32, f32),

    /// 中日字形回退按它挑字体，见 [`crate::FontLibrary::system`]。
    pub locale: String,
}

impl WindowOptions {
    pub fn new(title: impl Into<String>, size: (f32, f32)) -> Self {
        Self {
            title: title.into(),
            size,
            min_size: (size.0 * 0.6, size.1 * 0.5),
            locale: "zh-CN".to_owned(),
        }
    }
}
