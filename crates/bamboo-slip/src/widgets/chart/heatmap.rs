//! 格子图：一列一周、一格一天，颜色深浅是那天的量，像代码托管网站上的提交日历。

use std::hash::Hash;

use taffy::{NodeId, Style};

use super::{Datum, axis_label, placed, shade, tip};
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::{Interact, Ui};
use crate::widgets::WidgetTheme;
use crate::widgets::parts::{size, solid};

/// 格与格的间距（点）。
const GAP: f32 = 3.0;

/// 左边星期标签留的宽度（点）。
const ROW_LABEL_WIDTH: f32 = 26.0;

/// 上边月份标签留的高度（点）。
const COLUMN_LABEL_HEIGHT: f32 = 18.0;

pub struct Heatmap {
    /// 每列从上到下的格；`None` 是范围外的空位（不画）。
    columns: Vec<Vec<Option<Datum>>>,

    /// 格的边长（点）。
    cell: f32,

    /// 标在某几列上方的字（月份）。
    column_labels: Vec<(usize, String)>,

    /// 标在某几行左边的字（星期）。
    row_labels: Vec<(usize, String)>,
}

impl Heatmap {
    pub fn new(columns: Vec<Vec<Option<Datum>>>) -> Self {
        Self {
            columns,
            cell: 10.0,
            column_labels: Vec::new(),
            row_labels: Vec::new(),
        }
    }

    pub fn cell(mut self, points: f32) -> Self {
        self.cell = points;
        self
    }

    pub fn column_labels(mut self, labels: Vec<(usize, String)>) -> Self {
        self.column_labels = labels;
        self
    }

    pub fn row_labels(mut self, labels: Vec<(usize, String)>) -> Self {
        self.row_labels = labels;
        self
    }

    /// 放得下 `columns` 列时格子最大能多大（点），给使用方按可用宽度挑格子大小。
    pub fn fit(width: f32, columns: usize) -> f32 {
        let columns = columns.max(1) as f32;
        ((width - ROW_LABEL_WIDTH + GAP) / columns - GAP).max(4.0)
    }

    pub fn build<M>(
        self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        local: impl Hash,
    ) -> Result<NodeId, RenderError> {
        let scope = ui.key(local);
        let (cell, gap) = (theme.px(self.cell), theme.px(GAP));
        let step = cell + gap;
        let left = theme.px(ROW_LABEL_WIDTH);
        let top = theme.px(COLUMN_LABEL_HEIGHT);
        let rows = self.columns.iter().map(Vec::len).max().unwrap_or(0);
        let radius = theme.px(2.0);
        let mut children = Vec::new();
        for (column, label) in &self.column_labels {
            children.push(axis_label(
                ui,
                theme,
                label,
                placed(left + *column as f32 * step, 0.0),
            )?);
        }
        for (row, label) in &self.row_labels {
            let y = top + *row as f32 * step + (cell - theme.px(theme.small)) / 2.0;
            children.push(axis_label(
                ui,
                theme,
                label,
                placed(0.0, y - theme.px(1.0)),
            )?);
        }
        for (x, column) in self.columns.iter().enumerate() {
            for (y, datum) in column.iter().enumerate() {
                let Some(datum) = datum else {
                    continue;
                };
                let key = ui.key((scope, x, y));
                let hovered = ui.state(key).hovered;
                let border = hovered.then(|| (theme.px(1.0), theme.colors.text));
                let node = ui.scene().node(
                    Style {
                        size: size(Some(cell), Some(cell)),
                        ..placed(left + x as f32 * step, top + y as f32 * step)
                    },
                    solid(Some(shade(theme, datum.level)), border, radius),
                    &[],
                )?;
                ui.interact(node, key, Interact::new());
                tip(ui, theme, scope, key, &datum.tip)?;
                children.push(node);
            }
        }
        let width = left + self.columns.len() as f32 * step - gap;
        let height = top + rows as f32 * step - gap;
        let node = ui.scene().node(
            Style {
                size: size(Some(width), Some(height)),
                flex_shrink: 0.0,
                ..Style::default()
            },
            Visual::Group,
            &children,
        )?;
        // 登记整张图，下一帧才知道它在哪（说明框往哪边展开）
        ui.interact(node, scope, Interact::new());
        Ok(node)
    }
}
