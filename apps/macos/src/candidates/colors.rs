//! 六项自定义配色。普通词与生词独立设置，空值或非法色值回退各自默认色。

use objc2::rc::Retained;
use objc2_app_kit::NSColor;
use qingjian_platform::GeneralConfig;
use qingjian_render::{Color, Palette};

use super::theme::Theme;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CandidateColors {
    pub background: Option<Color>,

    pub text: Option<Color>,

    pub pos: Option<Color>,

    pub word: Option<Color>,

    pub fresh_word: Option<Color>,

    pub highlight: Option<Color>,
}

impl From<&GeneralConfig> for CandidateColors {
    fn from(config: &GeneralConfig) -> Self {
        Self {
            background: parse_hex(&config.candidate_background_color),
            text: parse_hex(&config.candidate_text_color),
            pos: parse_hex(&config.candidate_pos_color),
            word: parse_hex(&config.candidate_word_color),
            fresh_word: parse_hex(&config.candidate_fresh_word_color),
            highlight: parse_hex(&config.candidate_highlight_color),
        }
    }
}

impl CandidateColors {
    pub fn palette(&self, dark: bool) -> Palette {
        let mut palette = if dark {
            Palette::dark()
        } else {
            Palette::light()
        };
        if let Some(color) = self.background {
            palette.background = color;
        }
        if let Some(color) = self.text {
            palette.text = color;
            palette.cloud = color;
        }
        if let Some(color) = self.pos {
            palette.pos = color;
        }
        if let Some(color) = self.word {
            palette.gloss = color;
        }
        if let Some(color) = self.fresh_word {
            palette.fresh = color;
        }
        if let Some(color) = self.highlight {
            palette.highlight = color;
        }
        palette
    }

    pub fn apply_native(&self, theme: &mut Theme) {
        let defaults = Theme::system_default();
        let color = |custom: Option<Color>, default: Retained<NSColor>| {
            custom.map_or(default, native_color)
        };
        theme.background = color(self.background, defaults.background);
        theme.text_color = color(self.text, defaults.text_color);
        theme.cloud_color = color(self.text, defaults.cloud_color);
        theme.pos_color = color(self.pos, defaults.pos_color);
        theme.gloss_color = color(self.word, defaults.gloss_color);
        theme.fresh_color = color(self.fresh_word, defaults.fresh_color);
        theme.highlight = color(self.highlight, defaults.highlight);
    }
}

/// 接收六位 sRGB 或八位 RGBA；先检查 ASCII，避免手改配置中的 Unicode 导致切片 panic。
pub(crate) fn parse_hex(value: &str) -> Option<Color> {
    let hex = value.trim().strip_prefix('#')?;
    if !matches!(hex.len(), 6 | 8) || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some(Color::rgba(
        u8::from_str_radix(&hex[0..2], 16).ok()?,
        u8::from_str_radix(&hex[2..4], 16).ok()?,
        u8::from_str_radix(&hex[4..6], 16).ok()?,
        if hex.len() == 8 {
            u8::from_str_radix(&hex[6..8], 16).ok()?
        } else {
            255
        },
    ))
}

pub(crate) fn format_hex(color: Color) -> String {
    let rgb = format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b);
    if color.a == 255 {
        rgb
    } else {
        format!("{rgb}{:02X}", color.a)
    }
}

pub(crate) fn native_color(color: Color) -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(
        f64::from(color.r) / 255.0,
        f64::from(color.g) / 255.0,
        f64::from(color.b) / 255.0,
        f64::from(color.a) / 255.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_or_empty_colors_fall_back_without_changing_other_entries() {
        for value in ["", "#12", "#GG1122", "#中文", "123456", "#1234567"] {
            assert_eq!(parse_hex(value), None);
        }
        assert_eq!(parse_hex(" #aB12Ef "), Some(Color::rgb(171, 18, 239)));
        let translucent = Color::rgba(18, 52, 86, 120);
        assert_eq!(parse_hex("#12345678"), Some(translucent));
        assert_eq!(format_hex(translucent), "#12345678");
        for dark in [false, true] {
            let default = if dark {
                Palette::dark()
            } else {
                Palette::light()
            };
            assert_eq!(CandidateColors::default().palette(dark), default);
            let colors = CandidateColors::from(&GeneralConfig {
                candidate_background_color: "invalid".into(),
                candidate_pos_color: "#112233".into(),
                ..GeneralConfig::default()
            });
            let actual = colors.palette(dark);
            assert_eq!(actual.pos, Color::rgb(17, 34, 51));
            assert_eq!(actual.background, default.background);
            assert_eq!(actual.fresh, default.fresh);
            assert_eq!(actual.text, default.text);
        }
    }

    #[test]
    fn ordinary_and_fresh_colors_override_and_reset_independently() {
        let mut colors = CandidateColors {
            word: Some(Color::rgb(20, 80, 120)),
            ..CandidateColors::default()
        };
        for dark in [false, true] {
            let defaults = CandidateColors::default().palette(dark);
            let palette = colors.palette(dark);
            assert_eq!(palette.gloss, colors.word.unwrap());
            assert_eq!(palette.fresh, defaults.fresh);
            colors.fresh_word = Some(Color::rgb(140, 60, 180));
            let palette = colors.palette(dark);
            assert_eq!(palette.gloss, colors.word.unwrap());
            assert_eq!(palette.fresh, colors.fresh_word.unwrap());
            let fresh_only = CandidateColors {
                word: None,
                ..colors.clone()
            }
            .palette(dark);
            assert_eq!(fresh_only.gloss, defaults.gloss);
            assert_eq!(fresh_only.fresh, colors.fresh_word.unwrap());
            colors.fresh_word = None;
            assert_eq!(colors.palette(dark).fresh, defaults.fresh);
        }
    }
}
