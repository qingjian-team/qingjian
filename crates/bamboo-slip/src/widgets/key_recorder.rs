//! 录键按钮：平时显示当前的快捷键，点一下进入录制（显示提示），按下的键交给使用方判断收不收。
//! 录制与否由使用方保存（[`KeyRecorder::recording`]），收下、Esc、点了别处都由使用方结束录制。

use std::hash::Hash;

use taffy::{JustifyContent, NodeId, Style};

use super::WidgetTheme;
use super::parts::{DISABLED_OPACITY, focus_ring, label, over, padding, row, size, solid};
use crate::animation::Easing;
use crate::error::RenderError;
use crate::ui::{Interact, KeyCode, KeyPress, Ui};

pub struct KeyRecorder<'a> {
    /// 当前的快捷键（给人看的写法）。
    value: &'a str,

    /// 录制时显示的提示。
    prompt: &'a str,

    recording: bool,

    /// 宽度（点）；录制时文字换成提示，宽度固定才不跳。
    width: f32,

    disabled: bool,
}

impl<'a> KeyRecorder<'a> {
    pub fn new(value: &'a str) -> Self {
        Self {
            value,
            prompt: "按下新的组合键",
            recording: false,
            width: 150.0,
            disabled: false,
        }
    }

    pub fn prompt(self, prompt: &'a str) -> Self {
        Self { prompt, ..self }
    }

    pub fn recording(self, recording: bool) -> Self {
        Self { recording, ..self }
    }

    pub fn width(self, points: f32) -> Self {
        Self {
            width: points,
            ..self
        }
    }

    pub fn disabled(self, disabled: bool) -> Self {
        Self { disabled, ..self }
    }

    /// 点击产出 `on_start`；录制中按键产出 `on_press`（Tab 走焦点不算），Esc 与失去焦点产出 `on_cancel`。
    pub fn build<M: Clone + 'static>(
        self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        local: impl Hash,
        on_start: M,
        on_press: impl Fn(&KeyPress) -> M + 'static,
        on_cancel: M,
    ) -> Result<NodeId, RenderError> {
        let key = ui.key(local);
        let state = ui.state(key);
        let colors = &theme.colors;
        let radius = theme.px(theme.radius);
        let recording = self.recording && !self.disabled;
        let text = label(
            ui,
            theme,
            if recording { self.prompt } else { self.value },
            theme.font.size - 1.0,
            if recording { 400 } else { 500 },
            if recording {
                colors.secondary
            } else {
                colors.text
            },
        )?;
        let mut children = vec![text];
        if state.focus_ring {
            children.push(focus_ring(ui, theme, radius)?);
        }
        let fill = if recording {
            colors.surface
        } else if !self.disabled && state.pressed {
            over(colors.surface, colors.pressed)
        } else if !self.disabled && state.hovered {
            over(colors.surface, colors.hover)
        } else {
            colors.surface
        };
        let border = if recording {
            (theme.px(1.5), colors.accent)
        } else {
            (theme.px(1.0), colors.line)
        };
        let node = ui.scene().node(
            Style {
                padding: padding(theme, 0.0, 10.0),
                size: size(
                    Some(theme.px(self.width)),
                    Some(theme.px(theme.control_height)),
                ),
                justify_content: JustifyContent::CENTER,
                flex_shrink: 0.0,
                ..row(theme, 0.0)
            },
            solid(Some(fill), Some(border), radius),
            &children,
        )?;
        ui.fade(node, key, theme.hover, Easing::Ease);
        if self.disabled {
            ui.scene().set_opacity(node, DISABLED_OPACITY);
        }
        let mut interact = Interact::new()
            .on_click(on_start)
            .focusable()
            .disabled(self.disabled);
        if recording {
            let cancel = on_cancel.clone();
            interact = interact
                .on_key(move |press| {
                    Some(if press.key == KeyCode::Escape {
                        cancel.clone()
                    } else {
                        on_press(press)
                    })
                })
                .on_blur(on_cancel);
        }
        ui.interact(node, key, interact);
        ui.keyword(node, self.value);
        Ok(node)
    }
}
