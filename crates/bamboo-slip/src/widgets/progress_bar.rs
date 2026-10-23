//! 进度条：一条细轨道，左段按进度填强调色，撑满所在行的宽度。只是显示，不接交互。

use taffy::{Dimension, LengthPercentageAuto, NodeId, Position, Rect, Style};

use super::WidgetTheme;
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::Ui;

/// 轨道高（点）。
const HEIGHT: f32 = 6.0;

#[derive(Debug, Clone, Copy)]
pub struct ProgressBar {
    /// 0–1。
    fraction: f32,
}

impl ProgressBar {
    pub fn new(fraction: f32) -> Self {
        Self {
            fraction: fraction.clamp(0.0, 1.0),
        }
    }

    pub fn build<M>(self, ui: &mut Ui<M>, theme: &WidgetTheme) -> Result<NodeId, RenderError> {
        let colors = &theme.colors;
        let height = theme.px(HEIGHT);
        let radius = height / 2.0;
        // 宽 0 的填充 tiny_skia 画不了，会报 empty paths
        let mut fill = Vec::new();
        if self.fraction > 0.0 {
            fill.push(ui.scene().node(
                Style {
                    position: Position::Absolute,
                    size: taffy::Size {
                        width: Dimension::percent(self.fraction),
                        height: Dimension::length(height),
                    },
                    inset: Rect {
                        left: LengthPercentageAuto::length(0.0),
                        right: LengthPercentageAuto::auto(),
                        top: LengthPercentageAuto::length(0.0),
                        bottom: LengthPercentageAuto::auto(),
                    },
                    ..Style::default()
                },
                Visual::solid(colors.accent, radius),
                &[],
            )?);
        }
        ui.scene().node(
            Style {
                size: taffy::Size {
                    width: Dimension::percent(1.0),
                    height: Dimension::length(height),
                },
                flex_grow: 1.0,
                ..Style::default()
            },
            Visual::solid(colors.well, radius),
            &fill,
        )
    }
}
