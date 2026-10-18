//! 勾选框：方框加右边的文字，整块都能点；勾上时方框填强调色、画对勾。

use std::hash::Hash;
use std::sync::Arc;

use taffy::{NodeId, Style};

use super::WidgetTheme;
use super::check_mark::CheckMark;
use super::parts::{DISABLED_OPACITY, centered, focus_ring, label, over, row, solid};
use super::setting_row::spotlight;
use crate::animation::Easing;
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::{Interact, Ui};

/// 方框边长（点）。
const SIDE: f32 = 14.0;

#[derive(Debug, Clone, Copy)]
pub struct Checkbox<'a> {
    on: bool,

    label: &'a str,

    disabled: bool,
}

impl<'a> Checkbox<'a> {
    pub fn new(on: bool, label: &'a str) -> Self {
        Self {
            on,
            label,
            disabled: false,
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
        on_toggle: M,
    ) -> Result<NodeId, RenderError> {
        let key = ui.key(local);
        ui.searchable(key, (self.label, None), &[], &[]);
        let spot = spotlight(ui, theme, key, (4.0, 6.0))?;
        let state = ui.state(key);
        let colors = &theme.colors;
        let side = theme.px(SIDE);
        let radius = theme.px(4.0);
        let mut marks = Vec::new();
        if self.on {
            marks.push(ui.scene().node(
                centered(side, side),
                Visual::Icon {
                    icon: Arc::new(CheckMark),
                    size: side,
                    color: colors.on_accent,
                },
                &[],
            )?);
        }
        if state.focus_ring {
            marks.push(focus_ring(ui, theme, radius)?);
        }
        let hovered = state.hovered && !self.disabled;
        let (fill, border) = if self.on {
            let fill = if hovered {
                over(colors.accent, crate::color::Color::gray(255, 30))
            } else {
                colors.accent
            };
            (fill, None)
        } else {
            let fill = if hovered {
                over(colors.surface, colors.hover)
            } else {
                colors.surface
            };
            (fill, Some((theme.px(1.0), colors.secondary)))
        };
        let square = ui.scene().node(
            centered(side, side),
            solid(Some(fill), border, radius),
            &marks,
        )?;
        ui.fade(square, key.child("box"), theme.hover, Easing::Ease);
        let text = label(
            ui,
            theme,
            self.label,
            theme.font.size - 1.0,
            400,
            colors.text,
        )?;
        let node = ui.scene().node(
            Style { ..row(theme, 8.0) },
            crate::scene::Visual::Group,
            &[spot, square, text],
        )?;
        if self.disabled {
            ui.scene().set_opacity(node, DISABLED_OPACITY);
        }
        ui.interact(
            node,
            key,
            Interact::new()
                .on_click(on_toggle)
                .focusable()
                .disabled(self.disabled),
        );
        Ok(node)
    }
}
