//! `variables` 里的一个颜色值：只能直接写，不能再引用别的变量。

use serde::Deserialize;

use super::parse_hex;
use crate::color::Color;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct ColorValue(pub(crate) Color);

impl TryFrom<String> for ColorValue {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        parse_hex(&value)
            .map(Self)
            .ok_or_else(|| format!("invalid color {value:?}, expected #rrggbb or #rrggbbaa"))
    }
}
