//! 节点在根节点坐标里的矩形：状态条按各格的盒子分点击区域。

use taffy::NodeId;

use super::Scene;
use crate::error::RenderError;

impl Scene {
    /// `node` 在 `root` 坐标里的矩形（左、上、宽、高，像素），布局之后调。
    pub(crate) fn rect_in(
        &self,
        node: NodeId,
        root: NodeId,
    ) -> Result<(f32, f32, f32, f32), RenderError> {
        let (x, y) = self.parent_origin(node, root)?;
        let layout = self.tree.layout(node)?;
        Ok((
            x + layout.location.x,
            y + layout.location.y,
            layout.size.width,
            layout.size.height,
        ))
    }
}
