//! 候选窗口主题：字体、颜色、间距。所有可视参数集中在这里，方便以后从配置文件读。
//!
//! 视觉层级（产品决定）：候选词最深，译文稍浅，词性最浅，序号弱化。

use objc2::rc::Retained;
use objc2_app_kit::{NSColor, NSFont};

pub struct Theme {
    /// 候选词字体。
    pub text_font: Retained<NSFont>,

    /// 译文与词性字体。
    pub annotation_font: Retained<NSFont>,

    /// 序号字体。
    pub index_font: Retained<NSFont>,

    /// 候选词颜色。
    pub text_color: Retained<NSColor>,

    /// 译文颜色。
    pub gloss_color: Retained<NSColor>,

    /// 词性颜色，比译文更浅。
    pub pos_color: Retained<NSColor>,

    /// 生词译文的颜色：比普通译文醒目，提醒「这个词你还没见过几次」，看熟了就回到译文色。
    pub fresh_color: Retained<NSColor>,

    /// 序号颜色。
    pub index_color: Retained<NSColor>,

    /// 云联想的云朵与文字颜色：比译文醒目一点，但仍不抢候选词。
    pub cloud_color: Retained<NSColor>,

    /// 窗口背景。
    pub background: Retained<NSColor>,

    /// 当前候选的高亮底色。
    pub highlight: Retained<NSColor>,

    /// 窗口内边距。
    pub padding: f64,

    /// 行内上下留白。
    pub row_padding: f64,

    /// 序号与候选词、候选词与译文之间的间距。
    pub column_gap: f64,

    /// 窗口与高亮条的圆角。
    pub corner_radius: f64,

    /// 最多显示几行。
    pub max_rows: usize,
}

impl Theme {
    /// 系统默认外观。只能在主线程调用（NSFont / NSColor 不跨线程）。
    pub fn system_default() -> Self {
        Self {
            text_font: NSFont::systemFontOfSize(16.0),
            annotation_font: NSFont::systemFontOfSize(12.0),
            index_font: NSFont::systemFontOfSize(11.0),
            text_color: NSColor::labelColor(),
            gloss_color: NSColor::secondaryLabelColor(),
            pos_color: NSColor::tertiaryLabelColor(),
            fresh_color: NSColor::systemOrangeColor(),
            index_color: NSColor::tertiaryLabelColor(),
            cloud_color: NSColor::systemTealColor(),
            background: NSColor::windowBackgroundColor(),
            highlight: NSColor::colorWithSRGBRed_green_blue_alpha(0.0, 0.48, 1.0, 0.16),
            padding: 8.0,
            row_padding: 4.0,
            column_gap: 8.0,
            corner_radius: 8.0,
            max_rows: 9,
        }
    }
}
