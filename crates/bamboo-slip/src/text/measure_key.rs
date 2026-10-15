//! 量宽缓存的键：只含影响宽度的部分（颜色、描边、压线、gamma 不影响）。

use super::TextStyle;
use super::font::{FamilyId, FontWeight};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct MeasureKey {
    text: String,

    /// 字号、字号（点，字距表按它查）、行高的位模式。
    metrics: [u32; 3],

    weight: FontWeight,

    italic: bool,

    family: FamilyId,

    /// 折行宽度的位模式；不折行为 `None`。
    width: Option<u32>,
}

impl MeasureKey {
    pub(super) fn new(text: &str, style: &TextStyle, width: Option<f32>) -> Self {
        Self {
            text: text.to_owned(),
            metrics: [
                style.size.to_bits(),
                style.points.to_bits(),
                style.line_height.to_bits(),
            ],
            weight: style.weight,
            italic: style.italic,
            family: style.family,
            width: width.map(f32::to_bits),
        }
    }
}
