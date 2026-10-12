//! 颜色写法：`"#rrggbb"` / `"#rrggbbaa"`（sRGB，alpha 不预乘），或 `"@名字"` 引用 `variables` 里的颜色。

use serde::Deserialize;

use crate::color::Color;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "String")]
pub(crate) enum ColorRef {
    /// 直接写的颜色。
    Literal(Color),

    /// 引用的变量名（不含 `@`）。
    Variable(String),
}

impl TryFrom<String> for ColorRef {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if let Some(name) = value.strip_prefix('@') {
            return Ok(Self::Variable(name.to_owned()));
        }
        parse_hex(&value)
            .map(Self::Literal)
            .ok_or_else(|| format!("invalid color {value:?}, expected #rrggbb, #rrggbbaa or @name"))
    }
}

/// `#rrggbb` / `#rrggbbaa`。
pub(crate) fn parse_hex(value: &str) -> Option<Color> {
    let hex = value.strip_prefix('#')?;
    if !matches!(hex.len(), 6 | 8) || !hex.is_ascii() {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    let alpha = if hex.len() == 8 { byte(6)? } else { 255 };
    Some(Color::rgba(byte(0)?, byte(2)?, byte(4)?, alpha))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_and_variables() {
        assert_eq!(
            parse_hex("#b0ce7d7f"),
            Some(Color::rgba(176, 206, 125, 127))
        );
        assert_eq!(parse_hex("#1e1e1e"), Some(Color::rgb(30, 30, 30)));
        assert_eq!(parse_hex("#12345"), None);
        assert_eq!(
            ColorRef::try_from("@accent".to_owned()),
            Ok(ColorRef::Variable("accent".to_owned()))
        );
        assert!(ColorRef::try_from("red".to_owned()).is_err());
    }
}
