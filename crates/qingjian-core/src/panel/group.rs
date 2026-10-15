//! 面板的分组。

/// 面板的三组分类，按这个顺序排；搜索组只在敲的字搜得到东西时才有。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelGroup {
    Search,
    Symbols,
    Emoji,
}

impl PanelGroup {
    pub fn label(self) -> &'static str {
        match self {
            Self::Search => "搜索",
            Self::Symbols => "符号",
            Self::Emoji => "表情",
        }
    }

    pub(crate) fn from_label(label: &str) -> Option<Self> {
        match label {
            "符号" => Some(Self::Symbols),
            "表情" => Some(Self::Emoji),
            _ => None,
        }
    }
}
