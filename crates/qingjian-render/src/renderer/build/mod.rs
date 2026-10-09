//! 实例化：主题模板 + 一帧数据 → 场景树。节点按 `when` 取舍、按 `bind` 填数据，`use` 展开组件、`repeat` 按列表展开。
//!
//! 一个模板节点可能产出零个（条件不成立、绑定为空）或多个（`repeat`）场景节点，所以都往 `out` 里推。

mod cell;
mod context;
mod frame;
mod layout_style;
mod leaf;
mod row;

use taffy::NodeId;

use crate::animation::Keyframes;
use crate::error::RenderError;
use crate::frame::Frame;
use crate::layout::Layout;
use crate::renderer::StatusCell;
use crate::scene::{Effect, Scene, Visual};
use crate::text::{TextPainter, TextStyle};
use crate::theme::Theme;
use crate::theme::file::node::{BorderSpec, BoxSpec, EffectSpec, NodeKind, NodeSpec};
use crate::theme::file::{ColorSpec, FontRef};

use context::Context;

pub(super) struct Builder<'a> {
    pub(super) scene: &'a mut Scene,

    pub(super) theme: &'a Theme,

    /// 点 → 像素。
    pub(super) scale: f32,

    /// 量光标位置用。
    pub(super) text: &'a mut TextPainter,

    /// 状态条各格的盒子，按格的顺序；条件不成立没画出来的格为 `None`。
    pub(super) cell_nodes: Vec<Option<NodeId>>,

    /// 候选窗口里各候选产出的节点（第几个候选，节点）：点击区域取它们的外接矩形。
    pub(super) candidate_nodes: Vec<(usize, NodeId)>,

    /// 各候选译文里每条译词的片段节点（第几个候选，第几条，节点）。
    pub(super) sense_nodes: Vec<(usize, usize, NodeId)>,
}

impl Builder<'_> {
    /// 候选窗口的根节点。
    pub(super) fn window(&mut self, frame: &Frame, layout: Layout) -> Result<NodeId, RenderError> {
        let windows = &self.theme.file().windows;
        let spec = match layout {
            Layout::Vertical => &windows.vertical,
            Layout::Horizontal => &windows.horizontal,
        };
        self.root(spec, Context::new(frame))
    }

    /// 状态条的根节点（主题写了 `status.root`）：格子按 `cells` 展开，各格的盒子记在 [`Self::cell_nodes`]。
    pub(super) fn status(
        &mut self,
        spec: &NodeSpec,
        frame: &Frame,
        cells: &[StatusCell],
    ) -> Result<NodeId, RenderError> {
        self.root(spec, Context::status(frame, cells))
    }

    /// 实例化根模板；什么都没产出（条件不成立）时给一个空节点。
    fn root(&mut self, spec: &NodeSpec, ctx: Context) -> Result<NodeId, RenderError> {
        let mut out = Vec::with_capacity(1);
        self.node(spec, ctx, &BoxSpec::default(), &mut out)?;
        let root = match out.first() {
            Some(&root) => root,
            None => self
                .scene
                .node(taffy::Style::default(), Visual::Group, &[])?,
        };
        Ok(root)
    }

    /// 实例化一个模板节点；`over` 是外层 `use` 写的盒子属性，盖过这个节点自己的。
    fn node(
        &mut self,
        spec: &NodeSpec,
        ctx: Context,
        over: &BoxSpec,
        out: &mut Vec<NodeId>,
    ) -> Result<(), RenderError> {
        if spec.when.as_deref().is_some_and(|when| !ctx.holds(when)) {
            return Ok(());
        }
        let layout = spec.layout.overridden_by(over);
        let theme = self.theme;
        match &spec.kind {
            NodeKind::Frame { .. } => {
                self.frame(spec, &layout, ctx, out)?;
                self.mark_transition(spec, out);
                Ok(())
            }
            NodeKind::Use { component } => match theme.file().components.get(component) {
                Some(component) => self.node(component, ctx, &layout, out),
                None => Ok(()),
            },
            NodeKind::Repeat { bind, component } if bind == "cells" => {
                let Some(component) = theme.file().components.get(component) else {
                    return Ok(());
                };
                let cells = ctx.cells;
                for (i, cell) in cells.iter().enumerate() {
                    let before = out.len();
                    self.node(component, ctx.with_cell(cell, i, cells.len()), &layout, out)?;
                    // 每格第一个产出的节点就是这一格的盒子，点击按它的右边界分格
                    self.cell_nodes.push(out.get(before).copied());
                }
                Ok(())
            }
            NodeKind::Repeat { bind, component } => {
                let (Some(rows), Some(component)) =
                    (ctx.list(bind), theme.file().components.get(component))
                else {
                    return Ok(());
                };
                for (i, row) in rows.iter().enumerate() {
                    let row_ctx = ctx.with_row(row, i, rows.len());
                    let before = out.len();
                    self.node(component, row_ctx, &layout, out)?;
                    self.candidate_nodes
                        .extend(out[before..].iter().map(|&node| (i, node)));
                }
                Ok(())
            }
            _ => {
                if let Some(node) = self.leaf(spec, &layout, ctx)? {
                    out.push(node);
                    self.mark_transition(spec, out);
                }
                Ok(())
            }
        }
    }

    /// 节点写了 `id` 与 `transition`：登记到刚产出的场景节点上（`out` 的最后一个）；写了 `animation` 同样登记。
    fn mark_transition(&mut self, spec: &NodeSpec, out: &[NodeId]) {
        if let (Some(animation), Some(&node)) = (&spec.animation, out.last()) {
            self.scene
                .set_animation(node, Keyframes::from_spec(animation, self.scale));
        }
        if let (Some(id), Some(transition), Some(&node)) = (&spec.id, &spec.transition, out.last())
        {
            let duration =
                std::time::Duration::from_secs_f32(transition.duration.max(0.0) / 1000.0);
            self.scene
                .set_transition(node, id.clone(), duration, transition.easing);
        }
    }

    /// 盒子属性里的不透明度（作用于整棵子树）与效果设到节点上。
    fn apply_layer(&mut self, node: NodeId, layout: &BoxSpec, ctx: Context) {
        if let Some(opacity) = layout.opacity
            && opacity < 1.0
        {
            self.scene.set_opacity(node, opacity);
        }
        if let Some(effects) = &layout.effects {
            let effects = effects.iter().map(|spec| self.effect(spec, ctx)).collect();
            self.scene.set_effects(node, effects);
        }
    }

    /// 效果换成像素、颜色按当前数据与外观取值。
    fn effect(&self, spec: &EffectSpec, ctx: Context) -> Effect {
        Effect::from_spec(spec, self.scale, self.color(&spec.shadow().color, ctx))
    }

    /// 节点颜色：条件写法按当前数据取分支，再按外观取值。
    fn color(&self, spec: &ColorSpec, ctx: Context) -> crate::color::Color {
        let color = match spec {
            ColorSpec::Fixed(color) => color,
            ColorSpec::Switch {
                condition,
                then,
                otherwise,
            } => {
                if ctx.holds(condition) {
                    then
                } else {
                    otherwise
                }
            }
        };
        self.theme.color(color)
    }

    /// 命名文字样式按倍数换成像素、配上颜色与当前外观的 gamma。
    /// 文字的描边：宽度换成像素、颜色按当前数据与外观取值。
    fn stroke(
        &self,
        stroke: Option<&BorderSpec>,
        ctx: Context,
    ) -> Option<(f32, crate::color::Color)> {
        stroke.map(|stroke| (stroke.width * self.scale, self.color(&stroke.color, ctx)))
    }

    fn text_style(&self, font: &FontRef, color: crate::color::Color) -> TextStyle {
        let spec = self.theme.font_ref(font);
        TextStyle::new(
            spec.scaled(self.scale),
            spec.size,
            color,
            self.theme.text_gamma(),
        )
    }
}
