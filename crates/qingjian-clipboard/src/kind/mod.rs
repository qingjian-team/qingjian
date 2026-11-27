//! [`ClipKind`]：一条历史是什么内容。

mod filter;

pub use filter::ClipFilter;

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

#[cfg(test)]
mod tests {
    use super::ClipKind;

    #[test]
    fn key_matches_each_variant() {
        assert_eq!(ClipKind::Text.key(), "text");
        assert_eq!(ClipKind::Rich.key(), "rich");
        assert_eq!(ClipKind::Code.key(), "code");
        assert_eq!(ClipKind::Image.key(), "image");
        assert_eq!(ClipKind::Files.key(), "files");
    }

    #[test]
    fn parse_defaults_to_plain_text() {
        assert_eq!(ClipKind::parse("text"), ClipKind::Text);
        assert_eq!(ClipKind::parse("没见过的值"), ClipKind::Text);
        assert_eq!(ClipKind::parse(""), ClipKind::Text);
    }

    #[test]
    fn parse_round_trips_every_non_text_kind() {
        for kind in [
            ClipKind::Rich,
            ClipKind::Code,
            ClipKind::Image,
            ClipKind::Files,
        ] {
            assert_eq!(ClipKind::parse(kind.key()), kind);
        }
    }

    #[test]
    fn only_text_like_kinds_are_editable() {
        assert!(ClipKind::Text.editable());
        assert!(ClipKind::Rich.editable());
        assert!(ClipKind::Code.editable());
        assert!(!ClipKind::Image.editable());
        assert!(!ClipKind::Files.editable());
    }
}
