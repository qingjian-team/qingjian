//! 字重：100–900 的数字，或 `thin` / `light` / `regular` / `medium` / `semibold` / `bold` / `black` 这类名字。
//!
//! 可变字体（SF）按 `wght` 轴取值，多字重的字体集合（苹方、微软雅黑）挑最近的一档。

use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontWeight(pub u16);

impl FontWeight {
    pub const REGULAR: Self = Self(400);

    /// 名字 → 数值，与 CSS 的 `font-weight` 一致。
    fn from_name(name: &str) -> Option<Self> {
        let value = match name {
            "thin" => 100,
            "extralight" => 200,
            "light" => 300,
            "regular" | "normal" => 400,
            "medium" => 500,
            "semibold" => 600,
            "bold" => 700,
            "extrabold" => 800,
            "black" => 900,
            _ => return None,
        };
        Some(Self(value))
    }
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::REGULAR
    }
}

impl<'de> Deserialize<'de> for FontWeight {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Number(u16),
            Name(String),
        }
        match Raw::deserialize(deserializer)? {
            Raw::Number(value @ 1..=1000) => Ok(Self(value)),
            Raw::Number(value) => Err(serde::de::Error::custom(format!(
                "字重 {value} 不在 1–1000 之内"
            ))),
            Raw::Name(name) => Self::from_name(&name)
                .ok_or_else(|| serde::de::Error::custom(format!("不认识的字重 {name:?}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_numbers_and_names() {
        let parse = |json: &str| serde_json::from_str::<FontWeight>(json);
        assert_eq!(parse("600").unwrap(), FontWeight(600));
        assert_eq!(parse("\"bold\"").unwrap(), FontWeight(700));
        assert!(parse("0").is_err());
        assert!(parse("\"heavy\"").is_err());
    }
}
