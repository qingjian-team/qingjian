//! 节点在根节点坐标里的位置与树上的关系：点击区域、命中测试用。

use taffy::NodeId;

use super::Scene;
use super::located::{Located, Rect, intersect};
use crate::error::RenderError;

impl Scene {
    /// `node` 在 `root` 坐标里的矩形（左、上、宽、高，像素），布局之后调。
    pub fn rect_in(&self, node: NodeId, root: NodeId) -> Result<Rect, RenderError> {
        let (x, y) = self.parent_origin(node, root)?;
        let layout = self.tree.layout(node)?;
        Ok((
            x + layout.location.x,
            y + layout.location.y,
            layout.size.width,
            layout.size.height,
        ))
    }

    pub fn parent(&self, node: NodeId) -> Option<NodeId> {
        self.tree.parent(node)
    }

    /// 以 `root` 为根的所有节点按画序（先父后子、兄弟按先后），带在 `root` 坐标里的盒子与上级裁剪；布局之后调。
    pub fn paint_order(&self, root: NodeId) -> Result<Vec<Located>, RenderError> {
        let mut found = Vec::new();
        let mut stack: Vec<(NodeId, f32, f32, Option<Rect>)> = vec![(root, 0.0, 0.0, None)];
        while let Some((node, x, y, clip)) = stack.pop() {
            let layout = self.tree.layout(node)?;
            // 根节点自己的位置不算，与 `rect_in` 一致
            let (left, top) = if node == root {
                (0.0, 0.0)
            } else {
                (x + layout.location.x, y + layout.location.y)
            };
            let rect = (left, top, layout.size.width, layout.size.height);
            found.push(Located { node, rect, clip });
            let clips = self
                .tree
                .get_node_context(node)
                .is_some_and(|context| context.clip.is_some());
            let inner = match (clips, clip) {
                (true, Some(outer)) => Some(intersect(outer, rect)),
                (true, None) => Some(rect),
                (false, outer) => outer,
            };
            for child in self.tree.children(node)?.into_iter().rev() {
                stack.push((child, left, top, inner));
            }
        }
        Ok(found)
    }
}
