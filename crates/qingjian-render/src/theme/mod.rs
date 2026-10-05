//! 主题：字体、颜色、间距。所有可视参数都在这里，单位是点；将来从 TOML 读。
//!
//! 视觉层级（产品决定）：候选词最深，译文稍浅，词性最浅，序号弱化。数值对齐 macOS 壳的 AppKit 实现。

mod font_spec;
mod palette;

pub use font_spec::FontSpec;
pub use palette::Palette;

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
        Self::with_palette(Palette::light(), 0.85)
    }

    /// 深色，对齐 macOS 系统外观。
    pub fn dark() -> Self {
        Self::with_palette(Palette::dark(), 0.75)
    }

    fn with_palette(colors: Palette, text_gamma: f32) -> Self {
        Self {
            // 行高取 AppKit 系统字体在这几个字号下 NSAttributedString.size() 的高度
            text_font: FontSpec::new(16.0, 19.0),
            annotation_font: FontSpec::new(12.0, 15.0),
            index_font: FontSpec::new(11.0, 14.0),
            colors,
            padding: 8.0,
            row_padding: 4.0,
            column_gap: 8.0,
            corner_radius: 8.0,
            max_rows: 9,
            text_gamma,
        }
    }

    /// 按候选词字号缩放三种字体（译文与序号等比跟着缩放），间距与其余参数不变。
    /// 传入字号以缺省候选词字号（[`Theme::light`] 的 16 点）为基准。
    pub fn with_font_size(mut self, size: f32) -> Self {
        let scale = size / self.text_font.size;
        self.text_font = self.text_font.scaled(scale);
        self.annotation_font = self.annotation_font.scaled(scale);
        self.index_font = self.index_font.scaled(scale);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_size_scales_all_three_fonts_proportionally() {
        let theme = Theme::light().with_font_size(24.0);
        assert_eq!(theme.text_font, FontSpec::new(24.0, 28.5));
        assert_eq!(theme.annotation_font, FontSpec::new(18.0, 22.5));
        assert_eq!(theme.index_font, FontSpec::new(16.5, 21.0));
        // 间距与其余参数不动
        assert_eq!(theme.padding, 8.0);
        assert_eq!(theme.max_rows, 9);
    }
}
