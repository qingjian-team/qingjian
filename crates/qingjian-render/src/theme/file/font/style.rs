//! 正体 / 斜体。字体没有斜体面时由渲染器把正体倾斜 14° 代替。

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum FontStyle {
    #[default]
    Normal,

    Italic,
}

#[cfg(test)]
mod tests {
    use super::FontStyle;

    #[test]
    fn normal_is_the_default_and_both_names_are_lowercase() {
        assert_eq!(FontStyle::default(), FontStyle::Normal);
        assert_eq!(
            serde_json::from_str::<FontStyle>("\"normal\"").unwrap(),
            FontStyle::Normal
        );
        assert_eq!(
            serde_json::from_str::<FontStyle>("\"italic\"").unwrap(),
            FontStyle::Italic
        );
        assert!(serde_json::from_str::<FontStyle>("\"Oblique\"").is_err());
        assert!(serde_json::from_str::<FontStyle>("\"oblique\"").is_err());
    }
}
