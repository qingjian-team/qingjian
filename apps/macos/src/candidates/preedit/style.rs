use qingjian_core::MarkedKind;

/// preedit 片段的画法。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreeditStyle {
    /// 敲的拼音：正常深浅。
    Typed,

    /// 光标后剩下的拼音：淡一点。
    Rest,

    /// 被纠错改掉的字母：淡且带删除线。
    Struck,
}

impl From<MarkedKind> for PreeditStyle {
    fn from(kind: MarkedKind) -> Self {
        match kind {
            MarkedKind::Typed => Self::Typed,
            MarkedKind::Rest => Self::Rest,
            MarkedKind::Corrected => Self::Struck,
        }
    }
}
