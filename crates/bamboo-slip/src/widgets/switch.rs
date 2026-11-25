//! 开关：悬停时轨道加深，按下时旋钮拉宽，拨动时旋钮滑过去、轨道颜色渐变（同 macOS / iOS）。
//! [`Look::Fluent`] 照 WinUI 的 ToggleSwitch：关着是描边的空轨道和灰色小旋钮，悬停旋钮变大；左边可写「开 / 关」。

use std::hash::Hash;

use taffy::{LengthPercentageAuto, NodeId, Position, Rect, Style};

use super::WidgetTheme;
use super::parts::{DISABLED_OPACITY, focus_ring, label, lift, row, size, solid};
use super::theme::Look;
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::{Interact, Ui};

/// 轨道宽高、旋钮离轨道边的距离、按下时旋钮多拉出来的宽度（点）。
struct Metrics {
    track: (f32, f32),

    inset: f32,

    stretch: f32,
}

fn metrics(look: Look, hovered: bool) -> Metrics {
    match look {
        Look::Plain => Metrics {
            track: (36.0, 20.0),
            inset: 2.0,
            stretch: 5.0,
        },
        Look::Mac => Metrics {
            track: (32.0, 19.0),
            inset: 1.5,
            stretch: 4.0,
        },
        // 旋钮 12 点，悬停 14 点
        Look::Fluent => Metrics {
            track: (40.0, 20.0),
            inset: if hovered { 3.0 } else { 4.0 },
            stretch: 3.0,
        },
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Switch<'a> {
    on: bool,

    disabled: bool,

    /// 开着、关着时写在开关左边的字。
    labels: Option<(&'a str, &'a str)>,
}

impl<'a> Switch<'a> {
    pub fn new(on: bool) -> Self {
        Self {
            on,
            disabled: false,
            labels: None,
        }
    }

    pub fn disabled(self, disabled: bool) -> Self {
        Self { disabled, ..self }
    }

    /// 开关左边写当前状态（WinUI 的「开 / 关」）。
    pub fn labels(self, on: &'a str, off: &'a str) -> Self {
        Self {
            labels: Some((on, off)),
            ..self
        }
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
        let fluent = theme.look == Look::Fluent;
        let hovered = state.hovered && !self.disabled;
        let metrics = metrics(theme.look, hovered);
        let (width, height, inset) = (
            theme.px(metrics.track.0),
            theme.px(metrics.track.1),
            theme.px(metrics.inset),
        );
        let knob_height = height - inset * 2.0;
        let knob_width = if state.pressed {
            knob_height + theme.px(metrics.stretch)
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
            Visual::solid(
                if fluent && !self.on {
                    colors.secondary
                } else if fluent {
                    colors.on_accent
                } else {
                    colors.knob
                },
                knob_height / 2.0,
            ),
            &[],
        )?;
        if !fluent {
            ui.scene().set_effects(knob, vec![lift(theme)]);
        }
        ui.transition(knob, key.child("knob"), theme.flip, theme.flip_easing);

        let mut children = vec![knob];
        if state.focus_ring {
            children.push(focus_ring(ui, theme, height / 2.0)?);
        }
        let track = match (self.on, hovered) {
            (true, _) => colors.accent,
            (false, true) => colors.well_hover,
            (false, false) => colors.well,
        };
        // Fluent 关着时轨道只描一圈次要色的边
        let border = (fluent && !self.on).then(|| (theme.px(1.0), colors.secondary));
        let node = ui.scene().node(
            Style {
                size: size(Some(width), Some(height)),
                flex_shrink: 0.0,
                ..Style::default()
            },
            solid(Some(track), border, height / 2.0),
            &children,
        )?;
        ui.fade(node, key, theme.flip, crate::animation::Easing::Ease);
        let node = match self.labels {
            Some((on, off)) => {
                let text = if self.on { on } else { off };
                let text = label(ui, theme, text, theme.font.size, 400, colors.text)?;
                ui.scene()
                    .node(row(theme, 12.0), Visual::Group, &[text, node])?
            }
            None => node,
        };
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
