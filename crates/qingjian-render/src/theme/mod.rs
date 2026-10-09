//! 主题：字体、颜色、间距。所有可视参数都在这里，单位是点；将来从 TOML 读。
//!
//! 视觉层级（产品决定）：候选词最深，译文稍浅，词性最浅，序号弱化。数值对齐 macOS 壳的 AppKit 实现。

mod font_spec;
mod palette;

pub use font_spec::FontSpec;
pub use palette::Palette;

/// 候选字号的缺省值（点）；与 `[general] font_size` 缺省一致。
pub const DEFAULT_FONT_SIZE: f32 = 16.0;

/// 候选字号的合法范围（点）。
const FONT_SIZE_RANGE: std::ops::RangeInclusive<f32> = 8.0..=48.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    /// 候选词字体。
    pub text_font: FontSpec,

    /// 译文与词性字体。
    pub annotation_font: FontSpec,

    /// 序号字体。
    pub index_font: FontSpec,

    /// 配色。
    pub colors: Palette,

    /// 窗口内边距。
    pub padding: f32,

    /// 行内上下留白。
    pub row_padding: f32,

    /// 序号与候选词、候选词与译文之间的间距。
    pub column_gap: f32,

    /// 窗口与高亮条的圆角。
    pub corner_radius: f32,

    /// 最多显示几行。
    pub max_rows: usize,

    /// 文字抗锯齿覆盖率的 gamma：小于 1 笔画显粗。CoreText 对文字有一层类似的加深，深色背景上尤其明显，
    /// 线性混合出来的字会偏细；这个值按真机截图并排调。
    pub text_gamma: f32,
}

impl Theme {
    /// 浅色，对齐 macOS 系统外观。
    pub fn light() -> Self {
        Self::with_palette(Palette::light(), 0.85, DEFAULT_FONT_SIZE)
    }

    /// 深色，对齐 macOS 系统外观。
    pub fn dark() -> Self {
        Self::with_palette(Palette::dark(), 0.75, DEFAULT_FONT_SIZE)
    }

    /// 按候选字号取主题；越界夹回 [`FONT_SIZE_RANGE`]。
    pub fn with_font_size(dark: bool, font_size: f32) -> Self {
        if dark {
            Self::with_palette(Palette::dark(), 0.75, font_size)
        } else {
            Self::with_palette(Palette::light(), 0.85, font_size)
        }
    }

    /// 候选字号（点）；非有限数退回缺省。
    fn sanitized_font_size(font_size: f32) -> f32 {
        if font_size.is_finite() {
            font_size.clamp(*FONT_SIZE_RANGE.start(), *FONT_SIZE_RANGE.end())
        } else {
            DEFAULT_FONT_SIZE
        }
    }

    fn with_palette(colors: Palette, text_gamma: f32, font_size: f32) -> Self {
        // 行高取 AppKit 系统字体在这几个字号下 NSAttributedString.size() 的高度；
        // 字号按 `font_size` 等比缩放，保持候选/译文/序号三档的视觉层级不变。
        let scale = Self::sanitized_font_size(font_size) / DEFAULT_FONT_SIZE;
        Self {
            text_font: FontSpec::new(16.0 * scale, 19.0 * scale),
            annotation_font: FontSpec::new(12.0 * scale, 15.0 * scale),
            index_font: FontSpec::new(11.0 * scale, 14.0 * scale),
            colors,
            padding: 8.0,
            row_padding: 4.0,
            column_gap: 8.0,
            corner_radius: 8.0,
            max_rows: 9,
            text_gamma,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_themes_use_default_font_size() {
        assert_eq!(Theme::light().text_font.size, DEFAULT_FONT_SIZE);
        assert_eq!(Theme::dark().text_font.size, DEFAULT_FONT_SIZE);
    }

    #[test]
    fn font_size_scales_all_three_tiers_proportionally() {
        let base = Theme::light();
        let bigger = Theme::with_font_size(false, 20.0);
        let scale = 20.0 / DEFAULT_FONT_SIZE;
        assert_eq!(bigger.text_font.size, base.text_font.size * scale);
        assert_eq!(
            bigger.annotation_font.size,
            base.annotation_font.size * scale
        );
        assert_eq!(bigger.index_font.size, base.index_font.size * scale);
        // 行高同步缩放
        assert_eq!(
            bigger.text_font.line_height,
            base.text_font.line_height * scale
        );
    }

    #[test]
    fn font_size_out_of_range_or_non_finite_falls_back() {
        assert_eq!(
            Theme::with_font_size(false, 100.0).text_font.size,
            *FONT_SIZE_RANGE.end()
        );
        assert_eq!(
            Theme::with_font_size(false, 2.0).text_font.size,
            *FONT_SIZE_RANGE.start()
        );
        assert_eq!(
            Theme::with_font_size(false, f32::NAN).text_font.size,
            DEFAULT_FONT_SIZE
        );
    }
}
