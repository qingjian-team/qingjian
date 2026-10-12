//! 状态条的节点树画法：主题写了 `status.root` 时走这里，与候选窗口同一套模板、数据与绘制。
//! 格子由 `repeat` 绑定 `cells` 展开；点击区域按各格盒子的右边界分，格子之间夹着的装饰算前一格。

use super::{RenderedStatus, StatusCell};
use crate::error::RenderError;
use crate::frame::{Frame, Mode};
use crate::renderer::Renderer;
use crate::renderer::build::Builder;
use crate::scene::Scene;
use crate::theme::Theme;
use crate::theme::file::node::NodeSpec;

impl Renderer {
    pub(super) fn render_status_tree(
        &mut self,
        spec: &NodeSpec,
        cells: &[StatusCell],
        mode: &Mode,
        theme: &Theme,
        scale: f32,
    ) -> Result<RenderedStatus, RenderError> {
        let frame = Frame {
            mode: mode.clone(),
            ..Frame::default()
        };
        let mut scene = Scene::new();
        let mut builder = Builder {
            scene: &mut scene,
            theme,
            scale,
            text: &mut self.text,
            cell_nodes: Vec::new(),
            candidate_nodes: Vec::new(),
            sense_nodes: Vec::new(),
        };
        let root = builder.status(spec, &frame, cells)?;
        let cell_nodes = std::mem::take(&mut builder.cell_nodes);
        let (content_width, content_height) = scene.layout(root, &mut self.text)?;
        // 没画出来的格右边界同前一格（点不到它）；最后一格延到内容右边
        let mut edges = Vec::with_capacity(cell_nodes.len());
        let mut previous = 0.0_f32;
        for node in cell_nodes {
            if let Some(node) = node {
                let (x, _, width, _) = scene.rect_in(node, root)?;
                previous = previous.max(x + width);
            }
            edges.push(previous);
        }
        if let Some(last) = edges.last_mut() {
            *last = last.max(content_width.ceil());
        }
        let content = (content_width.ceil(), content_height.ceil());
        Ok(RenderedStatus {
            rendered: self.rasterize(&scene, root, content, scale)?,
            cell_edges: edges,
        })
    }
}
