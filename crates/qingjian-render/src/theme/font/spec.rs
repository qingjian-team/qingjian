//! 一种字体用法：字号、行高（点）、字重、斜体与字族。字族是主题字族表里的一条回退链，渲染器按装了哪些字体挑出实际用的。

use super::{FamilyId, FontWeight};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontSpec {
    /// 字号。
    pub size: f32,

    /// 行高：一行文字占的高度，字形在其中垂直居中。
    pub line_height: f32,

    pub weight: FontWeight,

    pub italic: bool,

    /// 字族回退链在主题字族表里的序号。
    pub(crate) family: FamilyId,
}

impl FontSpec {
    /// 缺省字族、常规字重的正体。
    pub const fn new(size: f32, line_height: f32) -> Self {
        Self {
            size,
            line_height,
            weight: FontWeight::REGULAR,
            italic: false,
            family: FamilyId(0),
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
