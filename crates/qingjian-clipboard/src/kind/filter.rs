//! [`ClipFilter`]：面板上按什么筛选。

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
