//! 输入框的外观：底色/描边/圆角、前置图标、聚焦下划线与外框节点。

use taffy::{AlignItems, LengthPercentageAuto, NodeId, Position, Rect, Size, Style};

use super::super::WidgetColors;
use super::super::parts::{DISABLED_OPACITY, centered, over, padding, row, size, solid};
use super::{MULTILINE_PADDING, TextField, WidgetTheme};
use crate::animation::Easing;
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::{ElementState, Key, Ui};

impl<'a> TextField<'a> {
    /// 外框节点：按单行/多行/浅底算出尺寸与底色，摆上图标、视口与聚焦下划线。
    pub(super) fn frame<M>(
        &self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        state: ElementState,
        focused: bool,
        multiline: bool,
        key: Key,
        view: NodeId,
    ) -> Result<NodeId, RenderError> {
        let colors = &theme.colors;
        let line_height = self.style(theme, colors.text).line_height;
        let tall = |rows: usize| rows.max(1) as f32 * line_height + theme.px(MULTILINE_PADDING * 2.0);
        let (height, min_height, vertical) = if multiline {
            (None, tall(self.rows), MULTILINE_PADDING)
        } else if self.filled {
            let extra = if theme.input_underline { 0.0 } else { 4.0 };
            (Some(theme.px(theme.control_height + extra)), 0.0, 0.0)
        } else {
            (Some(theme.px(theme.control_height)), 0.0, 0.0)
        };
        let max_height = match self.max_rows {
            Some(rows) if multiline => LengthPercentageAuto::length(tall(rows.max(self.rows))),
            _ => LengthPercentageAuto::auto(),
        };
        let paint = resolve(theme, colors, state, focused, self.filled);

        let mut children = Vec::new();
        if let Some(icon) = self.icon.clone() {
        let side = theme.px(13.0);
        children.push(ui.scene().node(
            centered(side, side),
            Visual::Icon {
                icon,
                size: side,
                color: colors.secondary,
            },
            &[],
        )?);
    }
    children.push(view);
    if theme.input_underline && focused {
        children.push(underline_bar(ui, theme, colors.accent)?);
    }

    let node = ui.scene().node(
        Style {
            padding: padding(theme, vertical, if self.filled { 9.0 } else { 8.0 }),
            size: size(self.width.map(|width| theme.px(width)), height),
            min_size: Size {
                width: LengthPercentageAuto::auto(),
                height: LengthPercentageAuto::length(min_height),
            },
            max_size: Size {
                width: LengthPercentageAuto::auto(),
                height: max_height,
            },
            flex_shrink: 0.0,
            align_items: if multiline {
                AlignItems::STRETCH
            } else {
                AlignItems::CENTER
            },
            ..row(theme, 7.0)
        },
        paint,
        &children,
    )?;
    ui.fade(node, key, theme.hover, Easing::Ease);
    if theme.input_underline {
        ui.scene().set_clip(node, theme.px(theme.radius));
    }
    if self.disabled {
        ui.scene().set_opacity(node, DISABLED_OPACITY);
    }
    Ok(node)
    }
}

/// 底色与描边：下划线款、浅底款、普通边框款三态按悬停/聚焦选色。
fn resolve(
    theme: &WidgetTheme,
    colors: &WidgetColors,
    state: ElementState,
    focused: bool,
    filled: bool,
) -> Visual {
    if theme.input_underline {
        let fill = if focused {
            colors.surface
        } else if state.hovered {
            over(colors.well, colors.hover)
        } else {
            colors.well
        };
        solid(
            Some(fill),
            Some((theme.px(1.0), colors.line)),
            theme.px(theme.radius),
        )
    } else if filled {
        let fill = if state.hovered && !focused {
            over(colors.well, colors.hover)
        } else {
            colors.well
        };
        let border = focused.then(|| (theme.px(1.5), colors.accent));
        solid(Some(fill), border, theme.px(theme.radius + 1.0))
    } else {
        let border = if focused { colors.accent } else { colors.line };
        solid(
            Some(colors.surface),
            Some((theme.px(if focused { 1.5 } else { 1.0 }), border)),
            theme.px(theme.radius),
        )
    }
}

/// 下划线款聚焦时底部那条强调色横条。
fn underline_bar<M>(
    ui: &mut Ui<M>,
    theme: &WidgetTheme,
    accent: crate::color::Color,
) -> Result<NodeId, RenderError> {
    ui.scene().node(
        Style {
            position: Position::Absolute,
            inset: Rect {
                left: LengthPercentageAuto::length(0.0),
                right: LengthPercentageAuto::length(0.0),
                bottom: LengthPercentageAuto::length(0.0),
                top: LengthPercentageAuto::auto(),
            },
            size: size(None, Some(theme.px(2.0))),
            ..Style::default()
        },
        solid(Some(accent), None, 0.0),
        &[],
    )
}
