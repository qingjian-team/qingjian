//! 一条历史是什么内容，以及面板上按什么筛选。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipKind {
    Text,

    /// 文字之外还带 HTML 或 RTF。
    Rich,

    Code,

    Image,

    Files,
}

impl ClipKind {
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Rich => "rich",
            Self::Code => "code",
            Self::Image => "image",
            Self::Files => "files",
        }
    }

    pub(crate) fn parse(key: &str) -> Self {
        match key {
            "rich" => Self::Rich,
            "code" => Self::Code,
            "image" => Self::Image,
            "files" => Self::Files,
            _ => Self::Text,
        }
    }

    /// 文字类的条目能编辑；图片和文件没有可改的文字。
    pub fn editable(self) -> bool {
        matches!(self, Self::Text | Self::Rich | Self::Code)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClipFilter {
    All,

    /// 纯文本和富文本。
    Text,

    Code,

    Image,

    Files,
}

impl ClipFilter {
    pub const ALL: [Self; 5] = [Self::All, Self::Text, Self::Code, Self::Image, Self::Files];

    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Text => "text",
            Self::Code => "code",
            Self::Image => "image",
            Self::Files => "files",
        }
    }
}
