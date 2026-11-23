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

#[cfg(test)]
mod tests {
    use super::ClipFilter;

    #[test]
    fn all_lists_every_filter() {
        assert_eq!(ClipFilter::ALL.len(), 5);
        assert_eq!(
            ClipFilter::ALL,
            [
                ClipFilter::All,
                ClipFilter::Text,
                ClipFilter::Code,
                ClipFilter::Image,
                ClipFilter::Files,
            ]
        );
    }

    #[test]
    fn keys_are_unique_and_lowercase() {
        let keys: Vec<&str> = ClipFilter::ALL.iter().map(|filter| filter.key()).collect();
        assert_eq!(keys, ["all", "text", "code", "image", "files"]);
        for key in keys {
            assert_eq!(key, key.to_ascii_lowercase());
        }
    }
}
