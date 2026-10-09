//! 表格一行里的各格按第一行文字的基线对齐。
//!
//! 主题用上边距按自己的字号把序号、候选、译文手调到同一条基线；设置里候选与译文按不同比例改了字号后，
//! 这里量出差多少，补到偏高那几格的上边距上。不用 taffy 自带的基线对齐：嵌套的框上报的基线是 0，且它把上边距也算进基线。

use std::collections::HashMap;

use taffy::{LengthPercentageAuto, NodeId, Position};

use super::{Scene, SceneNode, Visual};
use crate::error::RenderError;
use crate::text::TextSize;

impl Scene {
    /// 登记表格的一行，`cells` 不含横跨整行的格子。
    pub(crate) fn add_table_row(&mut self, cells: Vec<NodeId>) {
        if cells.len() > 1 {
            self.table_rows.push(cells);
        }
    }

    /// 把每行基线偏高的格子往下挪，返回挪没挪（挪了要重新布局）。差不到一像素的不动，按主题自己的字号显示时一格都不挪。
    pub(super) fn align_baselines(
        &mut self,
        measured: &HashMap<NodeId, TextSize>,
    ) -> Result<bool, RenderError> {
        let mut shifts = Vec::new();
        for row in &self.table_rows {
            let mut baselines = Vec::with_capacity(row.len());
            for &cell in row {
                if let Some(inner) = self.first_baseline(cell, measured)? {
                    baselines.push((cell, self.tree.layout(cell)?.location.y + inner));
                }
            }
            let lowest = baselines
                .iter()
                .map(|&(_, baseline)| baseline)
                .fold(f32::MIN, f32::max);
            for (cell, baseline) in baselines {
                let shift = (lowest - baseline).round();
                if shift >= 1.0 {
                    shifts.push((cell, shift));
                }
            }
        }
        for &(cell, shift) in &shifts {
            let top = self.tree.layout(cell)?.margin.top;
            let mut style = self.tree.style(cell)?.clone();
            style.margin.top = LengthPercentageAuto::length(top + shift);
            self.tree.set_style(cell, style)?;
        }
        Ok(!shifts.is_empty())
    }

    /// 节点里第一段文字的基线离节点顶边多远；绝对定位的子节点（高亮条这类叠层）不算。
    fn first_baseline(
        &self,
        node: NodeId,
        measured: &HashMap<NodeId, TextSize>,
    ) -> Result<Option<f32>, RenderError> {
        if let Some(SceneNode {
            visual: Visual::Text { .. },
            ..
        }) = self.tree.get_node_context(node)
        {
            return Ok(measured.get(&node).map(|size| size.baseline));
        }
        for child in self.tree.children(node)? {
            if self.tree.style(child)?.position == Position::Absolute {
                continue;
            }
            if let Some(inner) = self.first_baseline(child, measured)? {
                return Ok(Some(self.tree.layout(child)?.location.y + inner));
            }
        }
        Ok(None)
    }
}
