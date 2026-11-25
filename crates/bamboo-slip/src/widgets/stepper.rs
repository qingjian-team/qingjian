//! 步进框：显示数值，右边 − / + 两个小钮；整块拿焦点，↑↓ 也能调。数值的格式（单位、小数）由使用方定。
//! [`Look::Mac`] 是数值框右边上下叠着的一对小箭头（NSStepper）；[`Look::Fluent`] 是 NumberBox，上下钮在框里。

use std::hash::Hash;
use std::sync::Arc;

use taffy::{NodeId, Style};

use super::WidgetTheme;
use super::chevron::Chevron;
use super::parts::{
    DISABLED_OPACITY, centered, focus_ring, label, over, padding, row, size, solid,
};
use super::theme::Look;
use crate::animation::Easing;
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::{Interact, Key, KeyCode, Ui};

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
        let pair = (on_decrease.clone(), on_increase.clone());
        let node = match theme.look {
            Look::Plain => self.plain(ui, theme, key, pair)?,
            Look::Mac => self.mac(ui, theme, key, pair)?,
            Look::Fluent => self.fluent(ui, theme, key, pair)?,
        };
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

    /// 引擎缺省：数值框右边 − / + 两个钮。
    fn plain<M: Clone + 'static>(
        &self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        key: Key,
        (on_decrease, on_increase): (M, M),
    ) -> Result<NodeId, RenderError> {
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
        ui.scene().node(row(theme, 4.0), Visual::Group, &children)
    }

    /// macOS：右对齐的数值框，右边上下叠着两个小箭头钮。
    fn mac<M: Clone + 'static>(
        &self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        key: Key,
        (on_decrease, on_increase): (M, M),
    ) -> Result<NodeId, RenderError> {
        let state = ui.state(key);
        let colors = &theme.colors;
        let (height, radius) = (theme.px(22.0), theme.px(5.0));
        let hairline = theme.px(0.5).max(1.0);
        let value = label(ui, theme, self.value, theme.font.size, 400, colors.text)?;
        let mut field_children = vec![value];
        if state.focus_ring {
            field_children.push(focus_ring(ui, theme, radius)?);
        }
        let field = ui.scene().node(
            Style {
                padding: padding(theme, 0.0, 8.0),
                size: size(None, Some(height)),
                min_size: taffy::Size {
                    width: taffy::LengthPercentageAuto::length(theme.px(44.0)),
                    height: taffy::LengthPercentageAuto::auto(),
                },
                justify_content: taffy::JustifyContent::FLEX_END,
                ..row(theme, 0.0)
            },
            solid(Some(colors.surface), Some((hairline, colors.line)), radius),
            &field_children,
        )?;
        let shape = (theme.px(15.0), height / 2.0, 0.0);
        let up = self.arrow(
            ui,
            theme,
            (key.child("increase"), Chevron::Up),
            shape,
            (self.can_increase, on_increase),
        )?;
        let down = self.arrow(
            ui,
            theme,
            (key.child("decrease"), Chevron::Down),
            shape,
            (self.can_decrease, on_decrease),
        )?;
        let arrows = ui.scene().node(
            Style {
                flex_direction: taffy::FlexDirection::Column,
                ..Style::default()
            },
            solid(Some(colors.button), Some((hairline, colors.line)), radius),
            &[up, down],
        )?;
        ui.scene().set_clip(arrows, radius);
        ui.scene()
            .node(row(theme, 6.0), Visual::Group, &[field, arrows])
    }

    /// Fluent：NumberBox，左边数值，上下钮在框里靠右。
    fn fluent<M: Clone + 'static>(
        &self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        key: Key,
        (on_decrease, on_increase): (M, M),
    ) -> Result<NodeId, RenderError> {
        let state = ui.state(key);
        let colors = &theme.colors;
        let radius = theme.px(theme.radius);
        let value = ui.scene().node(
            Style {
                flex_grow: 1.0,
                ..Style::default()
            },
            Visual::Text {
                text: self.value.to_owned(),
                style: theme.text(theme.font.size, 400, colors.text),
            },
            &[],
        )?;
        let shape = (theme.px(26.0), theme.px(26.0), theme.px(3.0));
        let up = self.arrow(
            ui,
            theme,
            (key.child("increase"), Chevron::Up),
            shape,
            (self.can_increase, on_increase),
        )?;
        let down = self.arrow(
            ui,
            theme,
            (key.child("decrease"), Chevron::Down),
            shape,
            (self.can_decrease, on_decrease),
        )?;
        let mut children = vec![value, up, down];
        if state.focus_ring {
            children.push(focus_ring(ui, theme, radius)?);
        }
        ui.scene().node(
            Style {
                padding: taffy::Rect {
                    left: taffy::LengthPercentage::length(theme.px(11.0)),
                    right: taffy::LengthPercentage::length(theme.px(3.0)),
                    top: taffy::LengthPercentage::length(0.0),
                    bottom: taffy::LengthPercentage::length(0.0),
                },
                size: size(None, Some(theme.px(theme.control_height))),
                min_size: taffy::Size {
                    width: taffy::LengthPercentageAuto::length(theme.px(150.0)),
                    height: taffy::LengthPercentageAuto::auto(),
                },
                flex_shrink: 0.0,
                ..row(theme, 0.0)
            },
            solid(
                Some(colors.button),
                Some((theme.px(1.0), colors.line)),
                radius,
            ),
            &children,
        )
    }

    /// 一个箭头钮：`(宽, 高, 圆角)` 像素；到头时变淡、点了没反应。
    fn arrow<M: Clone + 'static>(
        &self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        (key, chevron): (Key, Chevron),
        (width, height, radius): (f32, f32, f32),
        (enabled, message): (bool, M),
    ) -> Result<NodeId, RenderError> {
        let colors = &theme.colors;
        let state = ui.state(key);
        let active = enabled && !self.disabled;
        let fill = (active && (state.hovered || state.pressed)).then_some(if state.pressed {
            colors.pressed
        } else {
            colors.hover
        });
        let glyph = theme.px(if theme.look == Look::Mac { 9.0 } else { 11.0 });
        let icon = ui.scene().node(
            centered(glyph, glyph),
            Visual::Icon {
                icon: Arc::new(chevron),
                size: glyph,
                color: colors.text,
            },
            &[],
        )?;
        let button =
            ui.scene()
                .node(centered(width, height), solid(fill, None, radius), &[icon])?;
        if !enabled {
            ui.scene().set_opacity(button, DISABLED_OPACITY);
        }
        ui.fade(button, key, theme.hover, Easing::Ease);
        ui.interact(
            button,
            key,
            Interact::new().on_click(message).disabled(!active),
        );
        Ok(button)
    }
}
