//! 悬浮状态条（Windows）的样式。不写 `root` 时格子的排法固定（每格内容居中、格间细线），用这里的尺寸与颜色；
//! 写了 `root` 就按节点树画（与候选窗口同一套写法），格子用 `repeat` 绑定 `cells`。

use serde::Deserialize;

use super::ColorRef;
use super::FontRef;
use super::node::{EffectSpec, NodeSpec};

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct StatusSpec {
    /// 格内文字的样式。
    pub(crate) font: FontRef,

    /// 每格左右的内边距，也决定条的高度（行高 + 内边距）。
    pub(crate) padding: f32,

    /// 圆角。
    pub(crate) radius: f32,

    pub(crate) background: ColorRef,

    /// 格间细线。
    pub(crate) separator: ColorRef,

    pub(crate) separator_width: f32,

    /// 普通文字。
    pub(crate) normal: ColorRef,

    /// 强调的文字（当前模式、生效中的全角标点）。
    pub(crate) emphasized: ColorRef,

    /// 齿轮。
    pub(crate) gear: ColorRef,

    pub(crate) gear_size: f32,

    /// 整条的投影、内阴影。
    #[serde(default)]
    pub(crate) effects: Vec<EffectSpec>,

    /// 节点树画法的根节点；写了它上面几项就不用了。
    #[serde(default)]
    pub(crate) root: Option<NodeSpec>,
}
