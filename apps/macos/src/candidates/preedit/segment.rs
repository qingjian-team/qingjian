use qingjian_core::MarkedSegment;

use super::style::PreeditStyle;

/// preedit 的一段文字及其画法。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreeditSegment {
    /// 文本。
    pub text: String,

    /// 画法。
    pub style: PreeditStyle,
}

impl From<&MarkedSegment> for PreeditSegment {
    fn from(segment: &MarkedSegment) -> Self {
        Self {
            text: segment.text.clone(),
            style: segment.kind.into(),
        }
    }
}
