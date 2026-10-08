//! 一种字体用法：字号、行高（点）、字族用途与加粗。

use super::FontRole;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontSpec {
    /// 字号。
    pub size: f32,

    /// 行高：一行文字占的高度，字形在其中垂直居中。
    pub line_height: f32,

    pub role: FontRole,

    pub bold: bool,
}

impl FontSpec {
    pub const fn new(size: f32, line_height: f32) -> Self {
        Self {
            size,
            line_height,
            role: FontRole::Ui,
            bold: false,
        }
    }

    /// 点 → 像素。
    pub(crate) fn scaled(self, scale: f32) -> Self {
        Self {
            size: self.size * scale,
            line_height: self.line_height * scale,
            ..self
        }
    }
}
