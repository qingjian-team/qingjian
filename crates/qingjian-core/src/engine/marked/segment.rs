use super::kind::MarkedKind;

/// preedit（marked text）的一段。整段 preedit 是若干段按顺序拼起来，光标位置按拼接后的字符数算。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkedSegment {
    /// 文本。
    pub text: String,

    /// 种类，决定样式。
    pub kind: MarkedKind,
}

impl MarkedSegment {
    pub fn new(text: impl Into<String>, kind: MarkedKind) -> Self {
        Self {
            text: text.into(),
            kind,
        }
    }
}
