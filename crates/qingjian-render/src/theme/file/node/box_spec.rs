//! 节点的盒子：外边距、内边距、尺寸、定位、对齐。都可以不写；`use` 引用组件时写的会盖过组件根节点上的。

use serde::Deserialize;

use super::align::Align;
use super::edges::Edges;
use super::effect::EffectSpec;
use super::position::Position;
use super::span::Span;

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub(crate) struct BoxSpec {
    pub(crate) margin: Option<Edges>,

    pub(crate) padding: Option<Edges>,

    /// 子节点之间的间距（点）。
    pub(crate) gap: Option<f32>,

    pub(crate) width: Option<f32>,

    pub(crate) height: Option<f32>,

    pub(crate) min_width: Option<f32>,

    pub(crate) position: Option<Position>,

    /// 绝对定位时四边到父框的距离，`[上, 右, 下, 左]`。
    pub(crate) inset: Option<Edges>,

    pub(crate) align_self: Option<Align>,

    /// 表格里跨列。
    pub(crate) span: Option<Span>,

    /// 不透明度（0–1），作用于节点连同子节点。
    pub(crate) opacity: Option<f32>,

    /// 投影、内阴影。
    pub(crate) effects: Option<Vec<EffectSpec>>,
}

impl BoxSpec {
    /// `over` 里写了的项盖过自己的。
    pub(crate) fn overridden_by(&self, over: &Self) -> Self {
        Self {
            margin: over.margin.or(self.margin),
            padding: over.padding.or(self.padding),
            gap: over.gap.or(self.gap),
            width: over.width.or(self.width),
            height: over.height.or(self.height),
            min_width: over.min_width.or(self.min_width),
            position: over.position.or(self.position),
            inset: over.inset.or(self.inset),
            align_self: over.align_self.or(self.align_self),
            span: over.span.or(self.span),
            opacity: over.opacity.or(self.opacity),
            effects: over.effects.clone().or_else(|| self.effects.clone()),
        }
    }
}
