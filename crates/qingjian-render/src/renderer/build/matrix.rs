//! 横排展开成矩阵：帧的 `columns` 不为 0 时，候选的 `repeat` 不直接排进外层那一行，而是包进一个网格，一行 `columns` 格。
//!
//! 主题不用为矩阵另写模板：每格照样用横排的候选组件，列间距沿用外层框的 `gap`。各列至少留帧给的字宽（壳按整份候选估的），
//! 滚动、移动高亮时窗口不跳；超出列宽的候选截尾加「…」，字宽按壳估列宽的同一套算（汉字一个字宽、ASCII 不到一个）。
//! 格子里不画逐项译文，否则各行高矮跟着内容变；主题写了 `highlighted.annotation` 的照常画在矩阵下面。

use bamboo_slip::RenderError;
use bamboo_slip::Visual;
use bamboo_slip::taffy::prelude::{TaffyAuto, TaffyGridLine, minmax};
use bamboo_slip::taffy::{
    AlignContent, AlignItems, Display, GridPlacement, LengthPercentage, Line,
    MaxTrackSizingFunction, MinTrackSizingFunction, NodeId, Size, Style,
};

use super::{Builder, Context};
use crate::frame::Row;
use crate::theme::file::node::{NodeKind, NodeSpec};

/// 帧没给列宽时每格最多几个字宽，与 Core 的 `MAX_CELL_EMS` 一致。
const MAX_CELL_EMS: f32 = 4.0;

/// ASCII 字符按几个字宽算，与 Core 估列宽的 `NARROW_EMS` 一致。
const NARROW_EMS: f32 = 0.62;

/// 云朵图标占几个字宽，与 Core 的 `CLOUD_EMS` 一致。
const CLOUD_EMS: f32 = 1.1;

impl Builder<'_> {
    /// `spec` 是要展开成矩阵的候选 `repeat` 时，建好网格返回；不是（或没展开）返回 `None`，照常实例化。
    pub(super) fn matrix(
        &mut self,
        spec: &NodeSpec,
        gap: Option<f32>,
        ctx: Context,
    ) -> Result<Option<NodeId>, RenderError> {
        let columns = ctx.frame.columns;
        let NodeKind::Repeat { bind, component } = &spec.kind else {
            return Ok(None);
        };
        if columns == 0
            || bind != "candidates"
            || spec.when.as_deref().is_some_and(|when| !ctx.holds(when))
        {
            return Ok(None);
        }
        let Some(component) = self.theme.file().components.get(component) else {
            return Ok(None);
        };
        let layout = spec.layout.clone();
        let shown: Vec<Row> = ctx
            .frame
            .rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let mut limit = ctx
                    .frame
                    .column_ems
                    .get(i % columns)
                    .copied()
                    .unwrap_or(MAX_CELL_EMS);
                if row.cloud {
                    limit -= CLOUD_EMS;
                }
                let mut row = row.clone();
                if let Some(text) = clipped(&row.text, limit) {
                    row.text = text;
                }
                row
            })
            .collect();
        let rows = &shown;
        let em = self.theme.font("candidate").size * self.scale;
        let mut cells = Vec::with_capacity(rows.len());
        for (i, row) in rows.iter().enumerate() {
            // 末行补的空位不建格子，免得画出分隔线这类装饰；格子按行列放，空着不影响别的格
            if row.text.is_empty() && row.index.is_empty() {
                continue;
            }
            let before = cells.len();
            self.node(
                component,
                ctx.with_row(row, i, rows.len()).in_matrix(),
                &layout,
                &mut cells,
            )?;
            let (grid_row, column) = (i / columns, i % columns);
            for &node in &cells[before..] {
                self.scene.place(
                    node,
                    Line {
                        start: GridPlacement::from_line_index(grid_row as i16 + 1),
                        end: GridPlacement::AUTO,
                    },
                    Line {
                        start: GridPlacement::from_line_index(column as i16 + 1),
                        end: GridPlacement::AUTO,
                    },
                    false,
                )?;
                self.candidate_nodes.push((i, node));
            }
        }
        let template = (0..columns)
            .map(|column| {
                let floor = ctx.frame.column_ems.get(column).map_or(0.0, |ems| ems * em);
                minmax(
                    MinTrackSizingFunction::length(floor),
                    MaxTrackSizingFunction::auto(),
                )
            })
            .collect();
        let gap = LengthPercentage::length(gap.unwrap_or(0.0) * self.scale);
        let style = Style {
            display: Display::Grid,
            grid_template_columns: template,
            gap: Size {
                width: gap,
                height: LengthPercentage::length(0.0),
            },
            align_content: Some(AlignContent::START),
            align_items: Some(AlignItems::START),
            justify_items: Some(AlignItems::START),
            ..Style::default()
        };
        Ok(Some(self.scene.node(style, Visual::Group, &cells)?))
    }
}

/// `text` 超过 `limit` 个字宽时截尾加「…」（「…」占一个字宽）；放得下返回 `None`。
fn clipped(text: &str, limit: f32) -> Option<String> {
    let width = |c: char| if c.is_ascii() { NARROW_EMS } else { 1.0 };
    // 估宽是浮点累加，留一点余量免得刚好放得下的也被截
    let limit = limit + 0.01;
    if text.chars().map(width).sum::<f32>() <= limit {
        return None;
    }
    let mut used = 1.0;
    let kept: String = text
        .chars()
        .take_while(|&c| {
            used += width(c);
            used <= limit
        })
        .collect();
    Some(format!("{kept}…"))
}

#[cfg(test)]
mod tests {
    use super::clipped;

    #[test]
    fn long_candidates_are_clipped_to_the_column() {
        assert_eq!(clipped("你好", 4.0), None);
        assert_eq!(clipped("你好世界", 4.0), None);
        assert_eq!(clipped("你好世界啊", 4.0).as_deref(), Some("你好世…"));
        assert_eq!(clipped("hello", 4.0), None);
        assert_eq!(clipped("historical", 4.0).as_deref(), Some("hist…"));
    }
}
