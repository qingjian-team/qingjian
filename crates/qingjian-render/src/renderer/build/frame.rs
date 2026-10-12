//! 框：flex 排子节点；带 `table` 时排成表格——`repeat` 出来的每份组件是一行，它的子节点依次是各列。
//!
//! 表格各列取各行最宽，最后补一列 `1fr` 吃掉剩余宽度，这样 `span: row` 的节点（高亮条）能横跨整个表格宽；
//! 列间距由各格的外边距给（grid 的 gap 会让补的那一列也多出一个间距）。

use taffy::prelude::{FromFr, TaffyAuto, TaffyGridLine, minmax};
use taffy::{
    AlignContent, AlignItems, Display, GridPlacement, GridTemplateComponent, JustifyContent, Line,
    MaxTrackSizingFunction, MinTrackSizingFunction, NodeId, Style,
};

use super::layout_style;
use super::{Builder, Context};
use crate::color::Color;
use crate::error::RenderError;
use crate::scene::{BoxPaint, Fill, Visual};
use crate::theme::file::node::{BoxSpec, FillSpec, NodeKind, NodeSpec, Span, StopSpec};

impl Builder<'_> {
    pub(super) fn frame(
        &mut self,
        spec: &NodeSpec,
        layout: &BoxSpec,
        ctx: Context,
        out: &mut Vec<NodeId>,
    ) -> Result<(), RenderError> {
        let NodeKind::Frame {
            direction,
            fill,
            border,
            radius,
            table,
            children,
        } = &spec.kind
        else {
            return Ok(());
        };
        let visual = if fill.is_some() || border.is_some() {
            Visual::Box(BoxPaint {
                fill: fill.as_ref().and_then(|fill| self.fill(fill, ctx)),
                border: border
                    .as_ref()
                    .map(|border| (border.width * self.scale, self.color(&border.color, ctx))),
                radius: radius * self.scale,
            })
        } else {
            Visual::Group
        };
        let base = layout_style::from_box(layout, self.scale);
        let node = match table {
            Some(table) => {
                let (cells, columns) = self.table_cells(children, ctx)?;
                let mut template = vec![GridTemplateComponent::AUTO; columns];
                template.push(GridTemplateComponent::from_fr(1.0));
                let style = Style {
                    display: Display::Grid,
                    grid_template_columns: template,
                    // 行高至少是主题写的（随设置里的候选字号缩放），内容更高时撑开，不会压到下一行
                    grid_auto_rows: vec![minmax(
                        MinTrackSizingFunction::length(
                            table.row_height * self.theme.candidate_scale() * self.scale,
                        ),
                        MaxTrackSizingFunction::auto(),
                    )],
                    justify_content: Some(JustifyContent::START),
                    align_content: Some(AlignContent::START),
                    align_items: Some(AlignItems::START),
                    justify_items: Some(AlignItems::START),
                    ..base
                };
                self.scene.node(style, visual, &cells)?
            }
            None => {
                let mut nodes = Vec::with_capacity(children.len());
                for child in children {
                    self.node(child, ctx, &BoxSpec::default(), &mut nodes)?;
                }
                self.scene
                    .node(layout_style::flex(base, *direction), visual, &nodes)?
            }
        };
        self.apply_layer(node, layout, ctx);
        out.push(node);
        Ok(())
    }

    /// 填充写法 → 画法：颜色按数据与外观解析，图片从主题素材取（没有就不画；SVG 不管 `scale`，它的单位就是点）。
    fn fill(&self, fill: &FillSpec, ctx: Context) -> Option<Fill> {
        let stops = |stops: &[StopSpec]| -> Vec<(Color, f32)> {
            let last = stops.len().saturating_sub(1).max(1) as f32;
            stops
                .iter()
                .enumerate()
                .map(|(i, stop)| match stop {
                    StopSpec::At(color, position) => (self.color(color, ctx), *position),
                    StopSpec::Even(color) => (self.color(color, ctx), i as f32 / last),
                })
                .collect()
        };
        Some(match fill {
            FillSpec::Color(color) => Fill::Solid(self.color(color, ctx)),
            FillSpec::Linear { linear, stops: s } => Fill::Linear {
                angle: *linear,
                stops: stops(s),
            },
            FillSpec::Radial { radial, stops: s } => Fill::Radial {
                center: *radial,
                stops: stops(s),
            },
            FillSpec::Image {
                image,
                slice,
                scale,
            } => match self.theme.svg(image) {
                Some(svg) => Fill::Svg {
                    image: svg,
                    slice: *slice,
                    px_per_unit: self.scale,
                },
                None => Fill::Image {
                    pixmap: self.theme.image(image)?,
                    slice: *slice,
                    pixels_per_px: scale / self.scale,
                },
            },
        })
    }

    /// 表格的格子（已放好行列）与列数。
    fn table_cells(
        &mut self,
        children: &[NodeSpec],
        ctx: Context,
    ) -> Result<(Vec<NodeId>, usize), RenderError> {
        let theme = self.theme;
        let mut cells = Vec::new();
        let mut columns = 0;
        for child in children {
            if child.when.as_deref().is_some_and(|when| !ctx.holds(when)) {
                continue;
            }
            let NodeKind::Repeat { bind, component } = &child.kind else {
                continue;
            };
            let (Some(rows), Some(component)) =
                (ctx.list(bind), theme.file().components.get(component))
            else {
                continue;
            };
            let NodeKind::Frame {
                children: row_cells,
                ..
            } = &component.kind
            else {
                continue;
            };
            for (i, row) in rows.iter().enumerate() {
                let row_ctx = ctx.with_row(row, i, rows.len());
                let grid_row = Line {
                    start: GridPlacement::from_line_index(i as i16 + 1),
                    end: GridPlacement::AUTO,
                };
                let mut column = 0;
                let mut aligned = Vec::with_capacity(row_cells.len());
                for cell in row_cells {
                    let mut built = Vec::with_capacity(1);
                    self.node(cell, row_ctx, &BoxSpec::default(), &mut built)?;
                    let span = cell.layout.span == Some(Span::Row);
                    for node in built {
                        let grid_column = if span {
                            Line {
                                start: GridPlacement::from_line_index(1),
                                end: GridPlacement::from_line_index(-1),
                            }
                        } else {
                            column += 1;
                            Line {
                                start: GridPlacement::from_line_index(column as i16),
                                end: GridPlacement::AUTO,
                            }
                        };
                        self.scene
                            .place(node, grid_row.clone(), grid_column, span)?;
                        if !span {
                            aligned.push(node);
                        }
                        self.candidate_nodes.push((i, node));
                        cells.push(node);
                    }
                }
                self.scene.add_table_row(aligned);
                columns = columns.max(column);
            }
        }
        Ok((cells, columns))
    }
}
