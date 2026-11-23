//! 节点在父框交叉轴上的对齐。

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Align {
    Start,

    End,

    Center,

    /// 撑满。
    Stretch,
}

#[cfg(test)]
mod tests {
    use super::Align;

    #[test]
    fn every_alignment_spelling_parses() {
        for (json, expected) in [
            ("\"start\"", Align::Start),
            ("\"end\"", Align::End),
            ("\"center\"", Align::Center),
            ("\"stretch\"", Align::Stretch),
        ] {
            assert_eq!(serde_json::from_str::<Align>(json).unwrap(), expected);
        }
    }

    #[test]
    fn alignment_names_are_lowercase_only() {
        assert!(serde_json::from_str::<Align>("\"Start\"").is_err());
        assert!(serde_json::from_str::<Align>("\"middle\"").is_err());
        assert!(serde_json::from_str::<Align>("1").is_err());
    }
}
