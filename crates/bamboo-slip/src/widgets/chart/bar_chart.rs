//! 柱状图：一根柱一个时段，高度是那段的量，同一个颜色（强调的那根与悬停的满色）；下面隔几根标一个刻度。

use std::hash::Hash;

use taffy::{NodeId, Style};

use super::{Datum, axis_label, placed, shade, tip};
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::{Interact, Ui};
use crate::widgets::WidgetTheme;
use crate::widgets::parts::{size, solid};

/// 下边刻度留的高度（点）。
const AXIS_HEIGHT: f32 = 18.0;

/// 柱宽占一格宽的比例，余下是间距。
const BAR_RATIO: f32 = 0.7;

pub struct BarChart {
    bars: Vec<Datum>,

    /// 柱区的宽与高（点），不含刻度。
    width: f32,

    height: f32,

    /// 标在某几根柱下面的刻度。
    labels: Vec<(usize, String)>,

    /// 强调的那根（比如现在这个钟点），画成满色。
    current: Option<usize>,

    /// 靠右边缘、右对齐的刻度（比如「现在」）。
    end_label: Option<String>,
}

impl BarChart {
    pub fn new(bars: Vec<Datum>, width: f32, height: f32) -> Self {
        Self {
            bars,
            width,
            height,
            labels: Vec::new(),
            current: None,
            end_label: None,
        }
    }

    pub fn end_label(mut self, label: impl Into<String>) -> Self {
        self.end_label = Some(label.into());
        self
    }

    pub fn labels(mut self, labels: Vec<(usize, String)>) -> Self {
        self.labels = labels;
        self
    }

    pub fn current(mut self, index: Option<usize>) -> Self {
        self.current = index;
        self
    }

    pub fn build<M>(
        self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        local: impl Hash,
    ) -> Result<NodeId, RenderError> {
        let scope = ui.key(local);
        let (width, height) = (theme.px(self.width), theme.px(self.height));
        let slot = width / self.bars.len().max(1) as f32;
        let bar = (slot * BAR_RATIO).max(1.0);
        let radius = theme.px(2.0).min(bar / 2.0);
        let mut children = Vec::new();
        // 底线
        children.push(ui.scene().node(
            Style {
                size: size(Some(width), Some(theme.px(1.0))),
                ..placed(0.0, height)
            },
            Visual::solid(theme.colors.line, 0.0),
            &[],
        )?);
        for (index, datum) in self.bars.iter().enumerate() {
            let key = ui.key((scope, index));
            let hovered = ui.state(key).hovered;
            let left = index as f32 * slot + (slot - bar) / 2.0;
            // 整根槽都能悬停，柱再矮也好指
            let level = datum.level.clamp(0.0, 1.0);
            let filled = if level > 0.0 {
                (height * level).max(theme.px(2.0))
            } else {
                0.0
            };
            let color = if self.current == Some(index) || hovered {
                theme.colors.accent
            } else {
                shade(theme, 0.5)
            };
            let mut fill = Vec::new();
            if filled > 0.0 {
                fill.push(ui.scene().node(
                    Style {
                        size: size(Some(bar), Some(filled)),
                        ..placed(0.0, height - filled)
                    },
                    solid(Some(color), None, radius),
                    &[],
                )?);
            }
            let node = ui.scene().node(
                Style {
                    size: size(Some(bar), Some(height)),
                    ..placed(left, 0.0)
                },
                Visual::Group,
                &fill,
            )?;
            ui.interact(node, key, Interact::new());
            tip(ui, theme, scope, key, &datum.tip)?;
            children.push(node);
        }
        for (index, label) in &self.labels {
            let x = *index as f32 * slot + (slot - bar) / 2.0;
            children.push(axis_label(
                ui,
                theme,
                label,
                placed(x, height + theme.px(4.0)),
            )?);
        }
        if let Some(label) = &self.end_label {
            let right = Style {
                inset: taffy::Rect {
                    left: taffy::LengthPercentageAuto::auto(),
                    right: taffy::LengthPercentageAuto::length(0.0),
                    top: taffy::LengthPercentageAuto::length(height + theme.px(4.0)),
                    bottom: taffy::LengthPercentageAuto::auto(),
                },
                ..placed(0.0, 0.0)
            };
            children.push(axis_label(ui, theme, label, right)?);
        }
        let node = ui.scene().node(
            Style {
                size: size(Some(width), Some(height + theme.px(AXIS_HEIGHT))),
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
