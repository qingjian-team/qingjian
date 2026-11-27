//! 一个长度（点）或 `"auto"`（只有外边距用得上，吃掉剩余空间）。

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(try_from = "LengthRepr")]
pub(crate) enum Length {
    Points(f32),

    Auto,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum LengthRepr {
    Number(f32),

    Word(String),
}

impl TryFrom<LengthRepr> for Length {
    type Error = String;

    fn try_from(value: LengthRepr) -> Result<Self, Self::Error> {
        match value {
            LengthRepr::Number(points) => Ok(Self::Points(points)),
            LengthRepr::Word(word) if word == "auto" => Ok(Self::Auto),
            LengthRepr::Word(word) => Err(format!(
                "invalid length {word:?}, expected a number or \"auto\""
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Length;

    #[test]
    fn numbers_and_auto_both_parse() {
        assert_eq!(
            serde_json::from_str::<Length>("12.5").unwrap(),
            Length::Points(12.5)
        );
        assert_eq!(
            serde_json::from_str::<Length>("0").unwrap(),
            Length::Points(0.0)
        );
        assert_eq!(
            serde_json::from_str::<Length>("\"auto\"").unwrap(),
            Length::Auto
        );
    }

    #[test]
    fn other_words_are_rejected_with_the_expected_forms() {
        let error = serde_json::from_str::<Length>("\"50%\"").unwrap_err();
        assert!(error.to_string().contains("auto"), "{error}");
        assert!(error.to_string().contains("50%"), "{error}");
        assert!(serde_json::from_str::<Length>("\"12\"").is_err());
        assert!(serde_json::from_str::<Length>("null").is_err());
        assert!(serde_json::from_str::<Length>("\"AUTO\"").is_err());
    }
}
