//! 符号面板的样式：排法固定（顶上分组胶囊与分类标签、中间等宽格子、底下信息行），这里给字体、尺寸与颜色。
//! 底、当前分组胶囊、高亮格是填充写法，能写渐变或图片。
//!
//! 字段按域拆成 [`PanelFonts`] / [`PanelMetrics`] / [`PanelPaint`]，序列化时用 `serde(flatten)`
//! 拍回原来的扁平键，主题文件的键结构不变。

use serde::Deserialize;

use super::ColorRef;
use super::FontRef;
use super::node::{EffectSpec, FillSpec};

/// 面板几处文字的样式引用。
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct PanelFonts {
    /// 分组胶囊、分类标签与信息行的样式。
    pub(crate) label_font: FontRef,

    /// 按键提示与行号的样式。
    pub(crate) hint_font: FontRef,

    /// 十格一行时格里条目的样式。
    pub(crate) item_font: FontRef,

    /// 格子拉宽时（颜文字）条目的样式。
    pub(crate) wide_item_font: FontRef,
}

/// 面板的尺寸、间距与圆角。
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct PanelMetrics {
    /// 四周的内边距。
    pub(crate) padding: f32,

    /// 整块的圆角。
    pub(crate) radius: f32,

    /// 格子的高，也是一行排满（十格）时每格的宽；格子区总宽按它算。
    pub(crate) cell: f32,

    pub(crate) cell_gap: f32,

    pub(crate) cell_radius: f32,

    /// 分组胶囊的左右、上下内边距与胶囊之间的间距。
    pub(crate) pill_padding_x: f32,

    pub(crate) pill_padding_y: f32,

    pub(crate) pill_gap: f32,

    /// 分类标签之间的间距与当前分类下划线的粗细。
    pub(crate) tab_gap: f32,

    pub(crate) underline: f32,

    /// 标签区与格子之间、格子与信息行之间的留白。
    pub(crate) section_gap: f32,

    /// 右上角关闭按钮的边长。
    pub(crate) close_size: f32,
}

/// 面板各处的填充与文字颜色。
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct PanelPaint {
    pub(crate) background: FillSpec,

    /// 条目、当前分类标签与信息行里的条目。
    pub(crate) text: ColorRef,

    /// 没选中的分组与分类、信息行里的名字。
    pub(crate) label: ColorRef,

    /// 按键提示、行号、标签放不下时的「‹」「›」。
    pub(crate) hint: ColorRef,

    pub(crate) pill_fill: FillSpec,

    pub(crate) pill_text: ColorRef,

    pub(crate) underline_color: ColorRef,

    /// 标签区下面的分隔线。
    pub(crate) divider: ColorRef,

    pub(crate) highlight: FillSpec,

    /// 关闭按钮的圆底与叉。
    pub(crate) close_fill: ColorRef,

    pub(crate) close_icon: ColorRef,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct PanelSpec {
    #[serde(flatten)]
    pub(crate) fonts: PanelFonts,

    #[serde(flatten)]
    pub(crate) metrics: PanelMetrics,

    #[serde(flatten)]
    pub(crate) paint: PanelPaint,

    /// 整块的投影。
    #[serde(default)]
    pub(crate) effects: Vec<EffectSpec>,
}
