//! 分段选择：几个互斥选项排成一条，选中项浮起一块底，换选项时这块底滑过去。
//! 整条拿焦点，←→ 换选项；单个选项能点不能单独拿焦点（同 macOS）。

use std::hash::Hash;

use taffy::{NodeId, Style};

use super::WidgetTheme;
use super::parts::{DISABLED_OPACITY, focus_ring, label, lift, overlay, padding, row, size, solid};
use crate::animation::Easing;
use crate::error::RenderError;
use crate::ui::{Interact, KeyCode, Ui};

/// 选项离槽边的距离（点）。
const INSET: f32 = 2.0;

#[derive(Debug, Clone, Copy)]
pub struct Segmented<'a> {
    options: &'a [&'a str],

    selected: usize,

    disabled: bool,
}

impl<'a> Segmented<'a> {
    pub fn new(options: &'a [&'a str], selected: usize) -> Self {
        Self {
            options,
            selected,
            disabled: false,
        }
    }

    pub fn disabled(self, disabled: bool) -> Self {
        Self { disabled, ..self }
    }

    /// `on_select(序号)` 在点选项或有焦点时按 ← / → 产出。
    pub fn build<M: Clone + 'static>(
        self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        local: impl Hash,
        on_select: impl Fn(usize) -> M,
    ) -> Result<NodeId, RenderError> {
        let key = ui.key(local);
        let state = ui.state(key);
        let colors = &theme.colors;
        let inset = theme.px(INSET);
        let height = theme.px(theme.control_height);
        let radius = theme.px(theme.radius);
        let messages: Vec<M> = (0..self.options.len()).map(&on_select).collect();

        let mut options = Vec::new();
        for (index, option) in self.options.iter().enumerate() {
            let option_key = key.child(index);
            let option_state = ui.state(option_key);
            let selected = index == self.selected;
            let color = if selected {
                colors.chip_text
            } else {
                colors.text
            };
            let mut children = Vec::new();
            if selected {
                let chip = ui.scene().node(
                    overlay(),
                    solid(Some(colors.chip), None, radius - inset),
                    &[],
                )?;
                ui.scene().set_effects(chip, vec![lift(theme)]);
                ui.transition(chip, key.child("chip"), theme.flip, theme.flip_easing);
                children.push(chip);
            }
            children.push(label(ui, theme, option, theme.font.size - 1.0, 500, color)?);
            let hover =
                !selected && !self.disabled && (option_state.hovered || option_state.pressed);
            let fill = hover.then_some(if option_state.pressed {
                colors.pressed
            } else {
                colors.hover
            });
            let node = ui.scene().node(
                Style {
                    padding: padding(theme, 0.0, 12.0),
                    size: size(None, Some(height - inset * 2.0)),
                    ..row(theme, 0.0)
                },
                solid(fill, None, radius - inset),
                &children,
            )?;
            ui.fade(node, option_key, theme.hover, Easing::Ease);
            ui.interact(
                node,
                option_key,
                Interact::new()
                    .on_click(messages[index].clone())
                    .disabled(self.disabled),
            );
            options.push(node);
        }
        if state.focus_ring {
            options.push(focus_ring(ui, theme, radius)?);
        }
        let node = ui.scene().node(
            Style {
                padding: padding(theme, INSET, INSET),
                flex_shrink: 0.0,
                ..row(theme, 0.0)
            },
            solid(Some(colors.well), None, radius),
            &options,
        )?;
        ui.anchor(node);
        if self.disabled {
            ui.scene().set_opacity(node, DISABLED_OPACITY);
        }
        let (selected, count) = (self.selected, self.options.len());
        ui.interact(
            node,
            key,
            Interact::new()
                .focusable()
                .disabled(self.disabled)
                .on_key(move |press| {
                    let next = match press.key {
                        KeyCode::ArrowLeft => selected.checked_sub(1)?,
                        KeyCode::ArrowRight => (selected + 1 < count).then_some(selected + 1)?,
                        _ => return None,
                    };
                    messages.get(next).cloned()
                }),
        );
        for option in self.options {
            ui.keyword(node, option);
        }
        Ok(node)
    }
}
