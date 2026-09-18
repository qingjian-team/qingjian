//! 一段文字怎么画：字号、行高（像素）、颜色、删除线。

use crate::color::Color;
use crate::theme::FontSpec;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TextStyle {
    /// 字号（像素）。
    pub size: f32,

    /// 字号（点）：光学字号与字距表按点查。
    pub points: f32,

    /// 行高（像素）。
    pub line_height: f32,

    pub color: Color,

    /// 画删除线（纠错改掉的拼音）。
    pub strike: bool,

    /// 画下划线（辅码码段）。
    pub underline: bool,

    /// 覆盖率 gamma，见 `Theme::text_gamma`。
    pub gamma: f32,
}

impl TextStyle {
    /// `font` 已按倍数换成像素；`points` 是换算前的字号。
    pub(crate) fn new(font: FontSpec, points: f32, color: Color, gamma: f32) -> Self {
        Self {
            size: font.size,
            points,
            line_height: font.line_height,
            color,
            strike: false,
            underline: false,
            gamma,
        }
    }

    pub(crate) fn struck(mut self) -> Self {
        self.strike = true;
        self
    }

    pub(crate) fn underlined(mut self) -> Self {
        self.underline = true;
        self
    }
}
