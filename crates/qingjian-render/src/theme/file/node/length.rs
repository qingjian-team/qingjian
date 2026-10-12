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
