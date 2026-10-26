//! 滑块：一条细轨道，左段填强调色，圆形旋钮按住拖、点轨道直接跳过去；右边显示数值（格式由使用方定）。
//! 整块拿焦点，← → ↑ ↓ 按步长调，Home / End 到两头。数值按步长取整，没变就不发消息，拖动时不会一直写配置。

use std::hash::Hash;
use std::rc::Rc;

use taffy::{AlignItems, LengthPercentageAuto, NodeId, Position, Rect, Style};

use super::WidgetTheme;
use super::parts::{DISABLED_OPACITY, focus_ring, label, lift, row, size};
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::{Interact, KeyCode, Ui};

/// 轨道的宽、高（点）。
const TRACK: (f32, f32) = (140.0, 4.0);

/// 旋钮直径（点），也是整条能点的高度。
const KNOB: f32 = 16.0;

/// 数值文字的最小宽度（点），数值变长短时滑块不跟着挪。
const VALUE_WIDTH: f32 = 40.0;

#[derive(Debug, Clone, Copy)]
pub struct Slider<'a> {
    value: f32,

    min: f32,

    max: f32,

    step: f32,

    /// 右边显示的数值；空着不显示。
    label: &'a str,

    disabled: bool,
}

impl<'a> Slider<'a> {
    pub fn new(value: f32, min: f32, max: f32) -> Self {
        Self {
            value,
            min,
            max,
            step: 1.0,
            label: "",
            disabled: false,
        }
    }

    pub fn step(self, step: f32) -> Self {
        Self { step, ..self }
    }

    pub fn label(self, label: &'a str) -> Self {
        Self { label, ..self }
    }

    pub fn disabled(self, disabled: bool) -> Self {
        Self { disabled, ..self }
    }

    /// 0–1 的位置。
    fn fraction(self) -> f32 {
        if self.max > self.min {
            ((self.value - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// `on_change` 收取整后的新值，与当前值相同时不调。
    pub fn build<M: Clone + 'static>(
        self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        local: impl Hash,
        on_change: impl Fn(f32) -> M + 'static,
    ) -> Result<NodeId, RenderError> {
        let key = ui.key(local);
        let state = ui.state(key);
        let colors = &theme.colors;
        let (width, track_height, knob) = (theme.px(TRACK.0), theme.px(TRACK.1), theme.px(KNOB));
        let travel = width - knob;
        let knob_left = travel * self.fraction();
        let track_top = (knob - track_height) / 2.0;
        let filled = knob_left + knob / 2.0;
        let absolute = |left: f32, top: f32, w: f32, h: f32| Style {
            position: Position::Absolute,
            size: size(Some(w), Some(h)),
            inset: Rect {
                left: LengthPercentageAuto::length(left),
                right: LengthPercentageAuto::auto(),
                top: LengthPercentageAuto::length(top),
                bottom: LengthPercentageAuto::auto(),
            },
            ..Style::default()
        };
        let radius = track_height / 2.0;
        let track = ui.scene().node(
            absolute(0.0, track_top, width, track_height),
            Visual::solid(colors.well, radius),
            &[],
        )?;
        let fill = ui.scene().node(
            absolute(0.0, track_top, filled, track_height),
            Visual::solid(colors.accent, radius),
            &[],
        )?;
        let knob_node = ui.scene().node(
            absolute(knob_left, 0.0, knob, knob),
            Visual::solid(colors.knob, knob / 2.0),
            &[],
        )?;
        ui.scene().set_effects(knob_node, vec![lift(theme)]);
        let mut children = vec![track, fill, knob_node];
        if state.focus_ring {
            children.push(focus_ring(ui, theme, knob / 2.0)?);
        }
        let bar = ui.scene().node(
            Style {
                size: size(Some(width), Some(knob)),
                flex_shrink: 0.0,
                ..Style::default()
            },
            Visual::Group,
            &children,
        )?;

        // 闭包要 'static，不能带着 `label` 的借用，只拿数值
        let on_change = Rc::new(on_change);
        let by_key = Rc::clone(&on_change);
        let Self {
            value: current,
            min,
            max,
            step,
            ..
        } = self;
        ui.interact(
            bar,
            key,
            Interact::new()
                .focusable()
                .disabled(self.disabled)
                .on_drag(move |fraction| {
                    let value = snap(min + fraction * (max - min), (min, max), step);
                    (value != current).then(|| on_change(value))
                })
                .on_key(move |press| {
                    let value = match press.key {
                        KeyCode::ArrowLeft | KeyCode::ArrowDown => current - step,
                        KeyCode::ArrowRight | KeyCode::ArrowUp => current + step,
                        KeyCode::Home => min,
                        KeyCode::End => max,
                        _ => return None,
                    };
                    let value = snap(value, (min, max), step);
                    (value != current).then(|| by_key(value))
                }),
        );

        let mut parts = vec![bar];
        if !self.label.is_empty() {
            let text = label(
                ui,
                theme,
                self.label,
                theme.font.size - 1.0,
                500,
                colors.text,
            )?;
            let value = ui.scene().node(
                Style {
                    min_size: taffy::Size {
                        width: LengthPercentageAuto::length(theme.px(VALUE_WIDTH)),
                        height: LengthPercentageAuto::auto(),
                    },
                    ..Style::default()
                },
                Visual::Group,
                &[text],
            )?;
            parts.push(value);
        }
        let node = ui.scene().node(
            Style {
                align_items: Some(AlignItems::CENTER),
                ..row(theme, 10.0)
            },
            Visual::Group,
            &parts,
        )?;
        if self.disabled {
            ui.scene().set_opacity(node, DISABLED_OPACITY);
        }
        Ok(node)
    }
}

/// 按步长取整（从下限数起）、夹在范围里。
fn snap(value: f32, (min, max): (f32, f32), step: f32) -> f32 {
    let steps = ((value - min) / step).round();
    (min + steps * step).clamp(min, max)
}
