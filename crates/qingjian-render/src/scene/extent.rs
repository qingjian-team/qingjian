//! 场景画出来会占多大：各节点的盒子（自己画东西的才算）加投影伸出的部分、文字描边，并在一起。
//! 伸出窗口的装饰（负 `inset` 的绝对定位节点）靠它把位图撑大，窗口本体仍是根节点的盒子。

use taffy::NodeId;

use super::{Effect, Scene, SceneNode, Visual};
use crate::error::RenderError;

/// 相对根节点左上角的范围（像素），总是包含根节点的盒子。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Extent {
    pub(crate) left: f32,

    pub(crate) top: f32,

    pub(crate) right: f32,

    pub(crate) bottom: f32,
}

impl Scene {
    /// 以 `root` 为根的整棵树画出来的范围；要在 [`Scene::layout`] 之后调。
    pub(crate) fn extent(&self, root: NodeId) -> Result<Extent, RenderError> {
        let size = self.tree.layout(root)?.size;
        let mut extent = Extent {
            left: 0.0,
            top: 0.0,
            right: size.width,
            bottom: size.height,
        };
        self.grow(root, 0.0, 0.0, &mut extent)?;
        Ok(extent)
    }

    /// 同 `grow`，给别的模块用。
    pub(super) fn grow_from(
        &self,
        node: NodeId,
        x: f32,
        y: f32,
        extent: &mut Extent,
    ) -> Result<(), RenderError> {
        self.grow(node, x, y, extent)
    }

    /// 把 `node`（父节点左上角在 `(x, y)`）及其子树并进 `extent`。
    fn grow(&self, node: NodeId, x: f32, y: f32, extent: &mut Extent) -> Result<(), RenderError> {
        let layout = self.tree.layout(node)?;
        let (x, y) = (x + layout.location.x, y + layout.location.y);
        if let Some(reach) = self.reach(node) {
            extent.include((x, y, layout.size.width, layout.size.height), reach);
        }
        for child in self.tree.children(node)? {
            self.grow(child, x, y, extent)?;
        }
        Ok(())
    }

    /// 节点画出来伸出盒子多远（投影、文字描边）；自己不画东西又没有效果的容器为 `None`。
    pub(crate) fn reach(&self, node: NodeId) -> Option<f32> {
        let SceneNode {
            visual, effects, ..
        } = self.tree.get_node_context(node)?;
        let shadow = effects.iter().map(Effect::overhang).fold(0.0, f32::max);
        let stroke = match visual {
            Visual::Text { style, .. } => style.stroke.map_or(0.0, |(width, _)| width),
            _ => 0.0,
        };
        (!matches!(visual, Visual::Group) || shadow > 0.0).then_some(shadow.max(stroke))
    }
}

impl Extent {
    /// 并进一个盒子（相对根节点）连同它伸出的 `reach`。
    pub(crate) fn include(&mut self, (x, y, width, height): (f32, f32, f32, f32), reach: f32) {
        self.left = self.left.min(x - reach);
        self.top = self.top.min(y - reach);
        self.right = self.right.max(x + width + reach);
        self.bottom = self.bottom.max(y + height + reach);
    }
}
