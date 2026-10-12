//! 主题里的节点：种类 + 显示条件 + 盒子属性。

mod align;
mod animation;
mod border;
mod box_spec;
mod caret;
mod direction;
mod edges;
mod effect;
mod fill;
mod kind;
mod length;
mod position;
mod span;
mod table;
mod tone_filter;
mod transition;

use serde::Deserialize;

pub(crate) use align::Align;
pub(crate) use animation::{AnimationSpec, KeyframeSpec};
pub(crate) use border::BorderSpec;
pub(crate) use box_spec::BoxSpec;
pub(crate) use direction::Direction;
pub(crate) use edges::Edges;
pub(crate) use effect::EffectSpec;
pub(crate) use fill::{FillSpec, StopSpec};
pub(crate) use kind::NodeKind;
pub(crate) use length::Length;
pub(crate) use position::Position;
pub(crate) use span::Span;
pub(crate) use tone_filter::ToneFilter;
pub(crate) use transition::TransitionSpec;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct NodeSpec {
    #[serde(flatten)]
    pub(crate) kind: NodeKind,

    /// 显示条件：数据字段名，`!` 取反，`a|b` 任一成立。不写总是显示。
    pub(crate) when: Option<String>,

    /// 过渡配对用的名字：新一帧与上一帧同 `id` 的节点之间插值。
    pub(crate) id: Option<String>,

    /// 写了才过渡，没写的节点即使有 `id` 也直接跳到新位置。
    pub(crate) transition: Option<TransitionSpec>,

    /// 循环动画（关键帧）。
    pub(crate) animation: Option<AnimationSpec>,

    #[serde(flatten)]
    pub(crate) layout: BoxSpec,
}
