//! 候选词和学习译词的独立排版设置，供 AppKit 与位图路径共用。

use super::native_font::NativeFont;
use objc2::MainThreadMarker;
use qingjian_platform::GeneralConfig;
use qingjian_render::{FontRole, FontSpec};

pub(crate) const MIN_FONT_SIZE: u16 = 8;
pub(crate) const MAX_FONT_SIZE: u16 = 48;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Typography {
    pub candidate_family: String,

    pub annotation_family: String,

    pub candidate_size: u16,

    pub annotation_size: u16,

    pub pos_size: u16,

    pub candidate_bold: bool,

    pub annotation_bold: bool,
}

impl From<&GeneralConfig> for Typography {
    fn from(config: &GeneralConfig) -> Self {
        Self {
            candidate_family: config.candidate_font.trim().to_owned(),
            annotation_family: config.font.trim().to_owned(),
            candidate_size: config
                .candidate_font_size
                .clamp(MIN_FONT_SIZE, MAX_FONT_SIZE),
            annotation_size: config
                .annotation_font_size
                .clamp(MIN_FONT_SIZE, MAX_FONT_SIZE),
            pos_size: config.pos_font_size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE),
            candidate_bold: config.candidate_bold,
            annotation_bold: config.annotation_bold,
        }
    }
}

impl Default for Typography {
    fn default() -> Self {
        Self::from(&GeneralConfig::default())
    }
}

impl Typography {
    pub fn native_fonts(&self, mtm: MainThreadMarker) -> (NativeFont, NativeFont, NativeFont) {
        (
            NativeFont::selected(
                mtm,
                &self.candidate_family,
                self.candidate_size,
                self.candidate_bold,
            ),
            NativeFont::selected(
                mtm,
                &self.annotation_family,
                self.annotation_size,
                self.annotation_bold,
            ),
            NativeFont::selected(
                mtm,
                &self.annotation_family,
                self.pos_size,
                self.annotation_bold,
            ),
        )
    }

    pub fn apply(&self, theme: &mut qingjian_render::Theme) {
        theme.text_font = spec(
            self.candidate_size,
            self.candidate_bold,
            FontRole::Candidate,
        );
        theme.annotation_font = spec(
            self.annotation_size,
            self.annotation_bold,
            FontRole::Annotation,
        );
        theme.pos_font = spec(self.pos_size, self.annotation_bold, FontRole::Annotation);
    }
}

fn spec(size: u16, bold: bool, role: FontRole) -> FontSpec {
    let size = f32::from(size);
    FontSpec {
        size,
        line_height: (size * 1.3).ceil(),
        role,
        bold,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_sizes_are_bounded_and_roles_are_independent() {
        let config = GeneralConfig {
            font: "Times New Roman".into(),
            candidate_font: "Songti SC".into(),
            candidate_font_size: 0,
            annotation_font_size: u16::MAX,
            pos_font_size: 0,
            candidate_bold: true,
            ..GeneralConfig::default()
        };
        let typography = Typography::from(&config);
        assert_eq!(typography.candidate_size, 8);
        assert_eq!(typography.annotation_size, 48);
        assert_eq!(typography.pos_size, 8);
        let mut theme = qingjian_render::Theme::light();
        typography.apply(&mut theme);
        assert_eq!(theme.text_font.role, FontRole::Candidate);
        assert_eq!(theme.annotation_font.role, FontRole::Annotation);
        assert_eq!(theme.pos_font.role, FontRole::Annotation);
        assert_eq!(theme.pos_font.size, 8.0);
        assert!(theme.text_font.bold);
        assert!(!theme.annotation_font.bold);
        assert_eq!(theme.index_font, qingjian_render::Theme::light().index_font);
    }

    #[test]
    fn pos_size_does_not_change_either_language_profile() {
        let mut config = GeneralConfig::default();
        let mut before = qingjian_render::Theme::light();
        Typography::from(&config).apply(&mut before);
        config.pos_font_size = 48;
        let mut after = qingjian_render::Theme::light();
        Typography::from(&config).apply(&mut after);
        assert_eq!(before.text_font, after.text_font);
        assert_eq!(before.annotation_font, after.annotation_font);
        assert_ne!(before.pos_font, after.pos_font);
    }
}
