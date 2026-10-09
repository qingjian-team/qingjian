//! 一段文字怎么画：字号、行高（像素）、字重、斜体、字族、颜色、描边、删除线。

use crate::color::Color;
use crate::theme::{FamilyId, FontSpec, FontWeight};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TextStyle {
    /// 字号（像素）。
    pub size: f32,

    /// 字号（点）：光学字号与字距表按点查。
    pub points: f32,

    /// 行高（像素）。
    pub line_height: f32,

    pub weight: FontWeight,

    /// 斜体；字体没有斜体面时倾斜正体代替。
    pub italic: bool,

    /// 字族回退链的序号，查当前主题的字族表。
    pub family: FamilyId,

    pub color: Color,

    /// 描边：向外的宽度（像素）与颜色；描边垫在字形底下，不占排版宽度。
    pub stroke: Option<(f32, Color)>,

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
            weight: font.weight,
            italic: font.italic,
            family: font.family,
            color,
            stroke: None,
            strike: false,
            underline: false,
            gamma,
        }
    }

    pub(crate) fn stroked(mut self, stroke: Option<(f32, Color)>) -> Self {
        self.stroke = stroke.filter(|(width, _)| *width > 0.0);
        self
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
