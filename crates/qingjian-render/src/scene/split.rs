//! 分段画：动画节点把树序切成几段，每段不动的内容各画进一张图层缓存起来；循环动画的每一帧只在变了的那块区域里
//! 按树序把「段 0、动画节点 1、段 1、……」叠一遍，只重画这一块。
//!
//! 节点按画的先后编号（进 `paint_subtree` 时取号），分段模式下只画号在范围里的节点；动画节点只占一个号、整棵子树跳过，
//! 另记下它的号与父节点左上角。容器拿子节点当阴影形状时临时退出分段模式，免得打乱编号。

use std::ops::Range;

use taffy::NodeId;

use super::Scene;
use crate::canvas::Canvas;
use crate::error::RenderError;
use crate::text::TextPainter;

/// 分段里跳过的一个动画节点：画的先后编号、父节点左上角（画布像素）。
#[derive(Debug, Clone, Copy)]
pub(crate) struct SplitMark {
    pub(crate) node: NodeId,

    pub(crate) order: usize,

    pub(crate) parent: (f32, f32),
}

impl Scene {
    /// 按分段模式画一遍 `root`：只画编号落在 `range` 里的节点，返回途中跳过的动画节点。
    pub(crate) fn paint_range(
        &self,
        root: NodeId,
        canvas: &mut Canvas,
        text: &mut TextPainter,
        origin: (f32, f32),
        range: Range<usize>,
    ) -> Result<Vec<SplitMark>, RenderError> {
        self.split.replace(Some(range));
        self.order.set(0);
        self.marks.borrow_mut().clear();
        let painted = self.paint(root, canvas, text, origin.0, origin.1);
        self.split.replace(None);
        painted?;
        Ok(self.marks.borrow_mut().drain(..).collect())
    }

    /// 分段模式下进一个节点：取号，返回这个节点自己要不要画。不在分段模式时总是画。
    pub(super) fn take_order(&self) -> bool {
        let range = self.split.borrow().clone();
        match range {
            Some(range) => {
                let order = self.order.get();
                self.order.set(order + 1);
                range.contains(&order)
            }
            None => true,
        }
    }

    /// 分段模式下遇到动画节点：占一个号、记下来，整棵子树跳过。返回是否跳过了。
    pub(super) fn skip_animated(&self, node: NodeId, parent: (f32, f32)) -> bool {
        if self.split.borrow().is_none() {
            return false;
        }
        let animated = self
            .tree
            .get_node_context(node)
            .is_some_and(|context| context.animation.is_some());
        if animated {
            let order = self.order.get();
            self.order.set(order + 1);
            self.marks.borrow_mut().push(SplitMark {
                node,
                order,
                parent,
            });
        }
        animated
    }

    /// 暂时退出分段模式跑 `f`（拿子节点当阴影形状时用），跑完恢复。
    pub(super) fn unsplit<T>(&self, f: impl FnOnce() -> T) -> T {
        let saved = self.split.replace(None);
        let order = self.order.get();
        let result = f();
        self.split.replace(saved);
        self.order.set(order);
        result
    }

    /// 动画节点能不能走局部重画：它的上级里没有动画节点、半透明容器、拿子节点当阴影形状的容器
    /// （这些上级的画面随它变），自己也不在过渡中。
    pub(crate) fn isolated(&self, node: NodeId) -> bool {
        let mut current = self.tree.parent(node);
        while let Some(parent) = current {
            let Some(context) = self.tree.get_node_context(parent) else {
                return false;
            };
            let group_with_effects =
                matches!(context.visual, super::Visual::Group) && !context.effects.is_empty();
            if context.animation.is_some() || context.opacity < 1.0 || group_with_effects {
                return false;
            }
            current = self.tree.parent(parent);
        }
        self.tree
            .get_node_context(node)
            .is_some_and(|context| context.transition.is_none())
    }
}
