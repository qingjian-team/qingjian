//! 统计图：按天的格子图（一列一周）与按时段的柱状图。数据由使用方折成 0–1 的深浅或高度，悬停时浮出各自的说明。

mod bar_chart;
mod datum;
mod heatmap;

pub use bar_chart::BarChart;
pub use datum::Datum;
pub use heatmap::Heatmap;

use taffy::{LengthPercentageAuto, NodeId, Position, Rect, Style};

use super::WidgetTheme;
use super::paragraph::paragraph;
use super::parts::{padding, solid};
use crate::color::Color;
use crate::error::RenderError;
use crate::scene::{Effect, EffectKind, Visual};
use crate::ui::{Key, Ui};

/// 0–1 的量折成四档深浅，0 是空槽的颜色。
fn shade(theme: &WidgetTheme, level: f32) -> Color {
    let colors = &theme.colors;
    if level <= 0.0 {
        return colors.well;
    }
    let step = (level.min(1.0) * 4.0).ceil() / 4.0;
    colors.well.lerp(colors.accent, 0.2 + 0.8 * step)
}

/// 指针停在 `key` 上时，在它上方浮出 `text`（浮层不被滚动区裁掉；第一帧还没位置，不出）。
/// 落在图 `chart` 右半边的往左展开，免得出了窗口。
fn tip<M>(
    ui: &mut Ui<M>,
    theme: &WidgetTheme,
    chart: Key,
    key: Key,
    text: &str,
) -> Result<(), RenderError> {
    if text.is_empty() || !ui.state(key).hovered {
        return Ok(());
    }
    let Some((x, y, width, _)) = ui.rect(key) else {
        return Ok(());
    };
    let right_half = ui
        .rect(chart)
        .is_some_and(|(left, _, chart_width, _)| x + width / 2.0 > left + chart_width / 2.0);
    let left = if right_half {
        x + width / 2.0 + theme.px(14.0) - estimated_width(theme, text)
    } else {
        x + width / 2.0 - theme.px(14.0)
    };
    let colors = &theme.colors;
    let line = paragraph(ui, theme, text, Some(colors.text))?;
    let bubble = ui.scene().node(
        Style {
            position: Position::Absolute,
            inset: Rect {
                left: LengthPercentageAuto::length(left),
                top: LengthPercentageAuto::length(y - theme.px(34.0)),
                right: LengthPercentageAuto::auto(),
                bottom: LengthPercentageAuto::auto(),
            },
            padding: padding(theme, 5.0, 8.0),
            ..Style::default()
        },
        solid(
            Some(colors.popup),
            Some((theme.px(1.0), colors.line)),
            theme.px(theme.radius),
        ),
        &[line],
    )?;
    ui.scene().set_effects(
        bubble,
        vec![Effect {
            kind: EffectKind::DropShadow,
            x: 0.0,
            y: theme.px(3.0),
            blur: theme.px(10.0),
            spread: 0.0,
            color: colors.shadow,
        }],
    );
    ui.overlay(bubble);
    Ok(())
}

/// 说明框大约多宽（像素）：建树时还量不了字，按汉字一个字宽、其余半个估。
fn estimated_width(theme: &WidgetTheme, text: &str) -> f32 {
    let ems: f32 = text
        .chars()
        .map(|c| if c.is_ascii() { 0.55 } else { 1.0 })
        .sum();
    theme.px(ems * theme.small + 16.0)
}

/// 绝对定位在 `(left, top)`（像素）的一个节点样式。
fn placed(left: f32, top: f32) -> Style {
    Style {
        position: Position::Absolute,
        inset: Rect {
            left: LengthPercentageAuto::length(left),
            top: LengthPercentageAuto::length(top),
            right: LengthPercentageAuto::auto(),
            bottom: LengthPercentageAuto::auto(),
        },
        ..Style::default()
    }
}

/// 一段次要色小字，标在图边上。
fn axis_label<M>(
    ui: &mut Ui<M>,
    theme: &WidgetTheme,
    text: &str,
    style: Style,
) -> Result<NodeId, RenderError> {
    ui.scene().node(
        style,
        Visual::Text {
            text: text.to_owned(),
            style: theme.text(theme.small - 1.0, 400, theme.colors.secondary),
        },
        &[],
    )
}
