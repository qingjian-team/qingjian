//! 一段文字怎么画：字号、行高（像素）、字重、斜体、字族、颜色、描边与压线。

use super::Decoration;
use super::font::{FamilyId, FontSpec, FontWeight};
use crate::color::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    /// 字号（像素）。
    pub size: f32,

    /// 字号（点）：光学字号与字距表按点查。
    pub points: f32,

    /// 行高（像素）。
    pub line_height: f32,

    pub weight: FontWeight,

    /// 斜体；字体没有斜体面时倾斜正体代替。
    pub italic: bool,

    /// 字族回退链的序号，查画笔当前的字族表。
    pub family: FamilyId,

    pub color: Color,

    /// 描边：向外的宽度（像素）与颜色；描边垫在字形底下，不占排版宽度。
    pub stroke: Option<(f32, Color)>,

    pub decoration: Option<Decoration>,

    /// 覆盖率 gamma：小于 1 把笔画加深，对齐 CoreText 的字体平滑。
    pub gamma: f32,

    /// 按节点宽度折行（段落）；不折行时一行到底，节点宽度就是文字宽度。
    pub wrap: bool,

    /// 一行放不下时截短并补「…」（节点被布局压得比文字窄时）；不截时照原样画出节点外。
    pub truncate: bool,
}

impl TextStyle {
    /// `font` 已按倍数换成像素；`points` 是换算前的字号。
    pub fn new(font: FontSpec, points: f32, color: Color, gamma: f32) -> Self {
        Self {
            size: font.size,
            points,
            line_height: font.line_height,
            weight: font.weight,
            italic: font.italic,
            family: font.family,
            color,
            stroke: None,
            decoration: None,
            gamma,
            wrap: false,
            truncate: false,
        }
    }

    pub fn wrapped(self) -> Self {
        Self { wrap: true, ..self }
    }

    pub fn truncated(self) -> Self {
        Self {
            truncate: true,
            ..self
        }
    }

    /// 宽度不大于 0 的描边当没有。
    pub fn stroked(self, stroke: Option<(f32, Color)>) -> Self {
        Self {
            stroke: stroke.filter(|(width, _)| *width > 0.0),
            ..self
        }
    }

    pub fn decorated(self, decoration: Decoration) -> Self {
        Self {
            decoration: Some(decoration),
            ..self
        }
    }
}
