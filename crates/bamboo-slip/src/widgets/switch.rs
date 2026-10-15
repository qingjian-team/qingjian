//! 开关：悬停时轨道加深，按下时旋钮拉宽，拨动时旋钮滑过去、轨道颜色渐变（同 macOS / iOS）。

use std::hash::Hash;

use taffy::{LengthPercentageAuto, NodeId, Position, Rect, Style};

use super::WidgetTheme;
use super::parts::{DISABLED_OPACITY, focus_ring, lift, size, solid};
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::{Interact, Ui};

/// 轨道宽高、旋钮离轨道边的距离（点）。
const TRACK: (f32, f32) = (36.0, 20.0);

const INSET: f32 = 2.0;

/// 按下时旋钮多拉出来的宽度（点）。
const STRETCH: f32 = 5.0;

#[derive(Debug, Clone, Copy)]
pub struct Switch {
    on: bool,

    disabled: bool,
}

impl Switch {
    pub fn new(on: bool) -> Self {
        Self {
            on,
            disabled: false,
        }
    }

    pub fn disabled(self, disabled: bool) -> Self {
        Self { disabled, ..self }
    }

    /// `on_toggle` 在点击或有焦点时按空格 / 回车产出。
    pub fn build<M: Clone + 'static>(
        self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        local: impl Hash,
        on_toggle: M,
    ) -> Result<NodeId, RenderError> {
        let key = ui.key(local);
        let state = ui.state(key);
        let colors = &theme.colors;
        let (width, height, inset) = (theme.px(TRACK.0), theme.px(TRACK.1), theme.px(INSET));
        let knob_height = height - inset * 2.0;
        let knob_width = if state.pressed {
            knob_height + theme.px(STRETCH)
        } else {
            knob_height
        };
        let left = if self.on {
            width - inset - knob_width
        } else {
            inset
        };
        let knob = ui.scene().node(
            Style {
                position: Position::Absolute,
                size: size(Some(knob_width), Some(knob_height)),
                inset: Rect {
                    left: LengthPercentageAuto::length(left),
                    right: LengthPercentageAuto::auto(),
                    top: LengthPercentageAuto::length(inset),
                    bottom: LengthPercentageAuto::auto(),
                },
                ..Style::default()
            },
            Visual::solid(colors.knob, knob_height / 2.0),
            &[],
        )?;
        ui.scene().set_effects(knob, vec![lift(theme)]);
        ui.transition(knob, key.child("knob"), theme.flip, theme.flip_easing);

        let mut children = vec![knob];
        if state.focus_ring {
            children.push(focus_ring(ui, theme, height / 2.0)?);
        }
        let track = match (self.on, state.hovered && !self.disabled) {
            (true, _) => colors.accent,
            (false, true) => colors.well_hover,
            (false, false) => colors.well,
        };
        let node = ui.scene().node(
            Style {
                size: size(Some(width), Some(height)),
                flex_shrink: 0.0,
                ..Style::default()
            },
            solid(Some(track), None, height / 2.0),
            &children,
        )?;
        ui.fade(node, key, theme.flip, crate::animation::Easing::Ease);
        ui.anchor(node);
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
