//! 部件模式在笔画序列里的匹配位置。

/// 模式在序列里的匹配位置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// 序列里任意一处（取最左一处）。
    Any,

    /// 只在序列开头。
    Prefix,

    /// 只在序列结尾。
    Suffix,

    /// 开头或结尾，先看结尾。
    Both,
}

impl Anchor {
    /// 解析覆盖表里写的锚点名，认不出返回 `None`。
    pub(super) fn parse(text: &str) -> Option<Self> {
        match text {
            "any" => Some(Self::Any),
            "prefix" => Some(Self::Prefix),
            "suffix" => Some(Self::Suffix),
            "both" => Some(Self::Both),
            _ => None,
        }
    }

    /// 按位置把序列里的一处 `from` 换成 `to`，没命中返回 `None`。
    pub(super) fn rewrite(self, seq: &str, from: &str, to: &str) -> Option<String> {
        match self {
            Self::Any => {
                let at = seq.find(from)?;
                Some(format!("{}{}{}", &seq[..at], to, &seq[at + from.len()..]))
            }
            Self::Prefix => seq.strip_prefix(from).map(|rest| format!("{to}{rest}")),
            Self::Suffix => seq.strip_suffix(from).map(|head| format!("{head}{to}")),
            Self::Both => Self::Suffix
                .rewrite(seq, from, to)
                .or_else(|| Self::Prefix.rewrite(seq, from, to)),
        }
    }
}
