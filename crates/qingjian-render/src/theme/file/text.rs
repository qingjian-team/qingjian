//! 文字设置：缺省字族、命名的文字样式（节点用 `"font": "名字"` 引用）与覆盖率 gamma。

use std::collections::HashMap;

use serde::Deserialize;

use super::adaptive::Adaptive;
use super::{FamilyList, FontStyle};
use crate::theme::FontWeight;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct TextSettings {
    /// 文字抗锯齿覆盖率的 gamma：小于 1 笔画显粗，见 `docs/design/rendering.md`。
    pub(crate) gamma: Adaptive<f32>,

    /// 没写字族的样式用的字族；不写为界面字体。
    #[serde(default)]
    pub(crate) family: Option<FamilyList>,

    /// 命名的文字样式。
    pub(crate) styles: HashMap<String, StyleSpec>,
}

/// 一个命名文字样式（点）。
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct StyleSpec {
    pub(crate) size: f32,

    /// 行高：一行文字占的高度，字形在其中垂直居中。
    pub(crate) line_height: f32,

    #[serde(default)]
    pub(crate) weight: FontWeight,

    /// 不写用 `text.family`。
    #[serde(default)]
    pub(crate) family: Option<FamilyList>,

    #[serde(default)]
    pub(crate) style: FontStyle,
}
