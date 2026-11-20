//! 键位提示键盘的样式（双拼、五笔字根）：三排字母键，排法固定（错位与真键盘一致），这里只给尺寸与颜色。
//!
//! 字段按域拆成 [`KeyboardFonts`] / [`KeyboardMetrics`] / [`KeyboardPaint`]，序列化时用
//! `serde(flatten)` 拍回原来的扁平键，主题文件的键结构不变。

use serde::Deserialize;

use super::ColorRef;
use super::FontRef;
use super::node::EffectSpec;

/// 键帽几处文字的样式引用。
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct KeyboardFonts {
    /// 键名（字母）的样式。
    pub(crate) key_font: FontRef,

    /// 翘舌声母、韵母、下一键音节的样式；五笔键底下一行的键名与区位号也用它。
    pub(crate) label_font: FontRef,

    /// 五笔字根的样式；格子放不下时按格子缩小。
    pub(crate) root_font: FontRef,
}

/// 键帽与整块键盘的尺寸、间距与圆角。
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct KeyboardMetrics {
    pub(crate) key_width: f32,

    pub(crate) key_height: f32,

    /// 五笔键（字根网格）的宽、高。
    pub(crate) root_key_width: f32,

    pub(crate) root_key_height: f32,

    /// 键与键、排与排之间的空隙。
    pub(crate) gap: f32,

    /// 键盘四周的内边距。
    pub(crate) padding: f32,

    /// 整块的圆角。
    pub(crate) radius: f32,

    pub(crate) key_radius: f32,

    /// 手机键盘整块（工具栏卡片、长按气泡）的圆角；不写就用手机壳自己的缺省。
    #[serde(default)]
    pub(crate) touch_radius: Option<f32>,

    /// 手机键盘每个键的圆角；不写就用手机壳自己的缺省。
    #[serde(default)]
    pub(crate) touch_key_radius: Option<f32>,
}

/// 键帽各状态的填充与文字颜色。
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct KeyboardPaint {
    pub(crate) background: ColorRef,

    pub(crate) key_fill: ColorRef,

    pub(crate) key_text: ColorRef,

    /// 韵母。
    pub(crate) label: ColorRef,

    /// 占键的翘舌声母（zh / ch / sh）。
    pub(crate) initial: ColorRef,

    /// 刚敲下、等着配第二键的那个键。
    pub(crate) pressed_fill: ColorRef,

    pub(crate) pressed_text: ColorRef,

    /// 能拼成音节的第二键。
    pub(crate) next_fill: ColorRef,

    pub(crate) next_text: ColorRef,

    /// 拼不出音节的键的文字。
    pub(crate) dimmed_text: ColorRef,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct KeyboardSpec {
    #[serde(flatten)]
    pub(crate) fonts: KeyboardFonts,

    #[serde(flatten)]
    pub(crate) metrics: KeyboardMetrics,

    #[serde(flatten)]
    pub(crate) paint: KeyboardPaint,

    /// 整块的投影。
    #[serde(default)]
    pub(crate) effects: Vec<EffectSpec>,
}
