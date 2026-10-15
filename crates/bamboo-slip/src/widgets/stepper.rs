//! 步进框：显示数值，右边 − / + 两个小钮；整块拿焦点，↑↓ 也能调。数值的格式（单位、小数）由使用方定。

use std::hash::Hash;

use taffy::{NodeId, Style};

use super::WidgetTheme;
use super::parts::{
    DISABLED_OPACITY, centered, focus_ring, label, over, padding, row, size, solid,
};
use crate::animation::Easing;
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::{Interact, KeyCode, Ui};

/// 数值框的最小宽度（点）。
const FIELD_WIDTH: f32 = 60.0;

#[derive(Debug, Clone, Copy)]
pub struct Stepper<'a> {
    value: &'a str,

    /// 到下限、上限时对应的钮不能点。
    can_decrease: bool,

    can_increase: bool,

    disabled: bool,
}

impl<'a> Stepper<'a> {
    pub fn new(value: &'a str) -> Self {
        Self {
            value,
            can_decrease: true,
            can_increase: true,
            disabled: false,
        }
    }

    pub fn limits(self, can_decrease: bool, can_increase: bool) -> Self {
        Self {
            can_decrease,
            can_increase,
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
        on_decrease: M,
        on_increase: M,
    ) -> Result<NodeId, RenderError> {
        let key = ui.key(local);
        let state = ui.state(key);
        let colors = &theme.colors;
        let height = theme.px(theme.control_height);
        let radius = theme.px(theme.radius);

        let value = label(
            ui,
            theme,
            self.value,
            theme.font.size - 1.0,
            500,
            colors.text,
        )?;
        let mut field_children = vec![value];
        if state.focus_ring {
            field_children.push(focus_ring(ui, theme, radius)?);
        }
        let field = ui.scene().node(
            Style {
                padding: padding(theme, 0.0, 10.0),
                size: size(None, Some(height)),
                min_size: taffy::Size {
                    width: taffy::LengthPercentageAuto::length(theme.px(FIELD_WIDTH)),
                    height: taffy::LengthPercentageAuto::auto(),
                },
                ..row(theme, 0.0)
            },
            solid(
                Some(colors.surface),
                Some((theme.px(1.0), colors.line)),
                radius,
            ),
            &field_children,
        )?;

        let mut buttons = Vec::new();
        for (name, glyph, enabled, message) in [
            ("decrease", "−", self.can_decrease, on_decrease.clone()),
            ("increase", "+", self.can_increase, on_increase.clone()),
        ] {
            let button_key = key.child(name);
            let button_state = ui.state(button_key);
            let active = enabled && !self.disabled;
            let fill = if active && button_state.pressed {
                over(colors.well, colors.pressed)
            } else if active && button_state.hovered {
                colors.well_hover
            } else {
                colors.well
            };
            let text = label(ui, theme, glyph, theme.font.size, 500, colors.text)?;
            let button = ui.scene().node(
                centered(height, height),
                solid(Some(fill), None, radius),
                &[text],
            )?;
            if !enabled {
                ui.scene().set_opacity(button, DISABLED_OPACITY);
            }
            ui.fade(button, button_key, theme.hover, Easing::Ease);
            ui.interact(
                button,
                button_key,
                Interact::new().on_click(message).disabled(!active),
            );
            buttons.push(button);
        }
        let mut children = vec![field];
        children.extend(buttons);
        let node = ui.scene().node(row(theme, 4.0), Visual::Group, &children)?;
        if self.disabled {
            ui.scene().set_opacity(node, DISABLED_OPACITY);
        }
        let (down, up) = (
            self.can_decrease.then_some(on_decrease),
            self.can_increase.then_some(on_increase),
        );
        ui.interact(
            node,
            key,
            Interact::new()
                .focusable()
                .disabled(self.disabled)
                .on_key(move |press| match press.key {
                    KeyCode::ArrowDown => down.clone(),
                    KeyCode::ArrowUp => up.clone(),
                    _ => None,
                }),
        );
        Ok(node)
    }
}
