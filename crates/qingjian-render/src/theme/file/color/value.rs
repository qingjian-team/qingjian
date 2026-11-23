//! `variables` 里的一个颜色值：只能直接写，不能再引用别的变量。

use serde::Deserialize;

use super::parse_hex;
use bamboo_slip::Color;

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

#[cfg(test)]
mod tests {
    use super::ColorValue;

    #[test]
    fn hex_colors_parse_with_optional_alpha() {
        let opaque: ColorValue = serde_json::from_str("\"#11aa22\"").unwrap();
        assert_eq!(
            (opaque.0.r, opaque.0.g, opaque.0.b, opaque.0.a),
            (0x11, 0xaa, 0x22, 255)
        );
        let translucent: ColorValue = serde_json::from_str("\"#11223380\"").unwrap();
        assert_eq!(translucent.0.a, 0x80);
    }

    #[test]
    fn malformed_colors_are_rejected_with_a_hint() {
        let error = serde_json::from_str::<ColorValue>("\"red\"").unwrap_err();
        assert!(error.to_string().contains("#rrggbb"), "{error}");
        assert!(serde_json::from_str::<ColorValue>("\"#fff\"").is_err());
        assert!(serde_json::from_str::<ColorValue>("\"#gggggg\"").is_err());
        assert!(serde_json::from_str::<ColorValue>("\"11aa22\"").is_err());
    }
}
