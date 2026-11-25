//! 分段选择：几个互斥选项排成一条，选中项浮起一块底，换选项时这块底滑过去。
//! 整条拿焦点，←→ 换选项；单个选项能点不能单独拿焦点（同 macOS）。
//! [`Look::Fluent`] 照 Windows Community Toolkit 的 Segmented：选中项描边加粗，底下一段强调色短条。

use std::hash::Hash;

use taffy::{NodeId, Style};

use super::WidgetTheme;
use super::parts::{DISABLED_OPACITY, focus_ring, label, lift, overlay, padding, row, size, solid};
use super::theme::Look;
use crate::animation::Easing;
use crate::error::RenderError;
use crate::ui::{Interact, KeyCode, Ui};

/// 选项离槽边的距离（点）。
const INSET: f32 = 2.0;

/// 一种外观下的尺寸（点）与字重。
struct Metrics {
    item_height: f32,

    padding: f32,

    min_width: f32,

    radius: f32,

    text: f32,

    weight: u16,

    selected_weight: u16,
}

fn metrics(theme: &WidgetTheme) -> Metrics {
    match theme.look {
        Look::Plain => Metrics {
            item_height: theme.control_height - INSET * 2.0,
            padding: 12.0,
            min_width: 0.0,
            radius: theme.radius,
            text: theme.font.size - 1.0,
            weight: 500,
            selected_weight: 500,
        },
        Look::Mac => Metrics {
            item_height: 20.0,
            padding: 10.0,
            min_width: 44.0,
            radius: 7.0,
            text: theme.font.size,
            weight: 400,
            selected_weight: 500,
        },
        Look::Fluent => Metrics {
            item_height: 28.0,
            padding: 12.0,
            min_width: 56.0,
            radius: 5.0,
            text: theme.font.size,
            weight: 400,
            selected_weight: 600,
        },
    }
}

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
        let metrics = metrics(theme);
        let fluent = theme.look == Look::Fluent;
        let radius = theme.px(metrics.radius);
        let messages: Vec<M> = (0..self.options.len()).map(&on_select).collect();

        let mut options = Vec::new();
        for (index, option) in self.options.iter().enumerate() {
            let option_key = key.child(index);
            let option_state = ui.state(option_key);
            let selected = index == self.selected;
            let color = match (selected, fluent) {
                (true, _) => colors.chip_text,
                (false, true) => colors.secondary,
                (false, false) => colors.text,
            };
            let mut children = Vec::new();
            if selected {
                let mut marks = Vec::new();
                if fluent {
                    marks.push(underline(ui, theme)?);
                }
                let border = fluent.then(|| (theme.px(1.0), colors.line));
                let chip = ui.scene().node(
                    overlay(),
                    solid(Some(colors.chip), border, radius - inset),
                    &marks,
                )?;
                if !fluent {
                    ui.scene().set_effects(chip, vec![lift(theme)]);
                }
                ui.transition(chip, key.child("chip"), theme.flip, theme.flip_easing);
                children.push(chip);
            }
            let weight = if selected {
                metrics.selected_weight
            } else {
                metrics.weight
            };
            children.push(label(ui, theme, option, metrics.text, weight, color)?);
            let hover =
                !selected && !self.disabled && (option_state.hovered || option_state.pressed);
            let fill = hover.then_some(if option_state.pressed {
                colors.pressed
            } else {
                colors.hover
            });
            let node = ui.scene().node(
                Style {
                    padding: padding(theme, 0.0, metrics.padding),
                    size: size(None, Some(theme.px(metrics.item_height))),
                    min_size: taffy::Size {
                        width: taffy::LengthPercentageAuto::length(theme.px(metrics.min_width)),
                        height: taffy::LengthPercentageAuto::auto(),
                    },
                    justify_content: taffy::JustifyContent::CENTER,
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
                ..row(theme, if fluent { INSET } else { 0.0 })
            },
            solid(
                Some(colors.well),
                fluent.then(|| (theme.px(1.0), colors.line)),
                radius,
            ),
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

/// Fluent 选中项底下的强调色短条：16 点宽、3 点高，离底边 2 点。
fn underline<M>(ui: &mut Ui<M>, theme: &WidgetTheme) -> Result<NodeId, RenderError> {
    let (width, height) = (theme.px(16.0), theme.px(3.0));
    let bar = ui.scene().node(
        Style {
            size: size(Some(width), Some(height)),
            ..Style::default()
        },
        solid(Some(theme.colors.accent), None, height / 2.0),
        &[],
    )?;
    ui.scene().node(
        Style {
            flex_direction: taffy::FlexDirection::Column,
            justify_content: taffy::JustifyContent::FLEX_END,
            align_items: taffy::AlignItems::CENTER,
            padding: padding(theme, 2.0, 0.0),
            ..overlay()
        },
        crate::scene::Visual::Group,
        &[bar],
    )
}
