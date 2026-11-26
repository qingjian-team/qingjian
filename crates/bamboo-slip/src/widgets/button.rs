//! 按钮：次按钮是带边框的浅底，主按钮是强调色底；悬停、按下换底色并渐变。

use std::hash::Hash;

use taffy::{NodeId, Style};

use super::WidgetTheme;
use super::parts::{DISABLED_OPACITY, focus_ring, label, over, padding, row, size, solid};
use super::theme::Look;
use crate::animation::Easing;
use crate::color::Color;
use crate::error::RenderError;
use crate::ui::{Interact, Ui};

#[derive(Debug, Clone, Copy)]
pub struct Button<'a> {
    label: &'a str,

    primary: bool,

    disabled: bool,
}

impl<'a> Button<'a> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            primary: false,
            disabled: false,
        }
    }

    /// 主按钮：一屏里最该点的那个。
    pub fn primary(self) -> Self {
        Self {
            primary: true,
            ..self
        }
    }

    pub fn disabled(self, disabled: bool) -> Self {
        Self { disabled, ..self }
    }

    pub fn build<M: Clone + 'static>(
        self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        local: impl Hash,
        on_click: M,
    ) -> Result<NodeId, RenderError> {
        let key = ui.key(local);
        let state = ui.state(key);
        let colors = &theme.colors;
        let active = !self.disabled;
        let (fill, text, border) = if self.primary {
            let fill = if active && state.pressed {
                over(colors.accent, Color::gray(0, 40))
            } else if active && state.hovered {
                over(colors.accent, Color::gray(255, 30))
            } else {
                colors.accent
            };
            (fill, colors.on_accent, None)
        } else {
            let fill = if active && state.pressed {
                over(colors.button, colors.pressed)
            } else if active && state.hovered {
                over(colors.button, colors.hover)
            } else {
                colors.button
            };
            // macOS 的按钮边只有半个点
            let edge = if theme.look == Look::Mac { 0.5 } else { 1.0 };
            (
                fill,
                colors.text,
                Some((theme.px(edge).max(1.0), colors.line)),
            )
        };
        let radius = theme.px(theme.radius);
        let (weight, side) = match theme.look {
            Look::Plain => (500, 14.0),
            Look::Mac => (400, 10.0),
            Look::Fluent => (400, 12.0),
        };
        let text = label(ui, theme, self.label, theme.font.size, weight, text)?;
        let mut children = vec![text];
        if state.focus_ring {
            children.push(focus_ring(ui, theme, radius)?);
        }
        let node = ui.scene().node(
            Style {
                padding: padding(theme, 0.0, side),
                size: size(None, Some(theme.px(theme.control_height))),
                flex_shrink: 0.0,
                ..row(theme, 0.0)
            },
            solid(Some(fill), border, radius),
            &children,
        )?;
        ui.fade(node, key, theme.hover, Easing::Ease);
        if self.disabled {
            ui.scene().set_opacity(node, DISABLED_OPACITY);
        }
        ui.interact(
            node,
            key,
            Interact::new()
                .on_click(on_click)
                .focusable()
                .disabled(self.disabled),
        );
        Ok(node)
    }
}
