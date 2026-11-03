//! 悬停提示：图标按钮这类没有文字的控件，指针停在上面时在它上方浮出一小段字。使用方判断悬停后调。
//! 是浮层（[`crate::Ui::overlay`]），位置贴着控件上一帧的位置，第一帧不出。

use taffy::{LengthPercentageAuto, NodeId, Position, Rect, Style};

use super::WidgetTheme;
use super::parts::{label, padding, solid};
use crate::error::RenderError;
use crate::scene::{Effect, EffectKind};
use crate::ui::{Key, Ui};

/// 提示与控件的距离（点）。
const GAP: f32 = 6.0;

/// 贴在 `anchor` 上方；控件在窗口右半边时右对齐，免得超出窗口。`window` 是根节点（窗口）的大小（像素）。
pub fn tooltip<M>(
    ui: &mut Ui<M>,
    theme: &WidgetTheme,
    anchor: Key,
    text: &str,
    window: (f32, f32),
) -> Result<Option<NodeId>, RenderError> {
    let Some((x, y, width, _)) = ui.rect(anchor) else {
        return Ok(None);
    };
    let colors = &theme.colors;
    let content = label(ui, theme, text, theme.small, 400, colors.text)?;
    let (left, right) = if x > window.0 / 2.0 {
        (
            LengthPercentageAuto::auto(),
            LengthPercentageAuto::length(window.0 - x - width),
        )
    } else {
        (
            LengthPercentageAuto::length(x),
            LengthPercentageAuto::auto(),
        )
    };
    let bubble = ui.scene().node(
        Style {
            position: Position::Absolute,
            inset: Rect {
                left,
                right,
                top: LengthPercentageAuto::auto(),
                bottom: LengthPercentageAuto::length(window.1 - y + theme.px(GAP)),
            },
            padding: padding(theme, 5.0, 8.0),
            ..Style::default()
        },
        solid(
            Some(colors.popup),
            Some((theme.px(1.0), colors.line)),
            theme.px(theme.radius),
        ),
        &[content],
    )?;
    ui.scene().set_effects(
        bubble,
        vec![Effect {
            kind: EffectKind::DropShadow,
            x: 0.0,
            y: theme.px(2.0),
            blur: theme.px(8.0),
            spread: 0.0,
            color: colors.shadow,
        }],
    );
    ui.overlay(bubble);
    Ok(Some(bubble))
}
