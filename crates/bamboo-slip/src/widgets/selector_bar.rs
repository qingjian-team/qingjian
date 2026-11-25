//! 选择栏（WinUI 的 SelectorBar）：切换同一页的几种视图，几个文字项排成一行，没有槽；
//! 选中项字加粗、底下居中一段强调色短条，换选项时短条滑过去；悬停的项垫一块浅底。
//! 整条拿焦点，←→ 换选项。不分外观，哪种外观用它由使用方定。

use std::hash::Hash;

use taffy::{AlignItems, FlexDirection, JustifyContent, NodeId, Style};

use super::WidgetTheme;
use super::parts::{focus_ring, label, overlay, padding, row, size, solid};
use crate::animation::Easing;
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::{Interact, KeyCode, Ui};

/// 一项的高度（点）。
const HEIGHT: f32 = 40.0;

/// 一项左右的留白（点）。
const PADDING: f32 = 12.0;

/// 项与项之间（点）。
const SPACING: f32 = 4.0;

/// 悬停底色的圆角（点）。
const RADIUS: f32 = 4.0;

/// 短条的宽、高与离底边的距离（点）。
const MARK: (f32, f32, f32) = (16.0, 3.0, 4.0);

#[derive(Debug, Clone, Copy)]
pub struct SelectorBar<'a> {
    options: &'a [&'a str],

    selected: usize,
}

impl<'a> SelectorBar<'a> {
    pub fn new(options: &'a [&'a str], selected: usize) -> Self {
        Self { options, selected }
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
        let radius = theme.px(RADIUS);
        let messages: Vec<M> = (0..self.options.len()).map(&on_select).collect();

        let mut items = Vec::new();
        for (index, option) in self.options.iter().enumerate() {
            let item_key = key.child(index);
            let item_state = ui.state(item_key);
            let selected = index == self.selected;
            let mut children = Vec::new();
            if selected {
                let mark = mark(ui, theme)?;
                ui.transition(mark, key.child("mark"), theme.flip, theme.flip_easing);
                children.push(mark);
            }
            let (weight, color) = if selected {
                (600, colors.text)
            } else {
                (400, colors.secondary)
            };
            children.push(label(ui, theme, option, theme.font.size, weight, color)?);
            let fill = if item_state.pressed {
                Some(colors.pressed)
            } else if item_state.hovered {
                Some(colors.hover)
            } else {
                None
            };
            let node = ui.scene().node(
                Style {
                    padding: padding(theme, 0.0, PADDING),
                    size: size(None, Some(theme.px(HEIGHT))),
                    justify_content: JustifyContent::CENTER,
                    flex_shrink: 0.0,
                    ..row(theme, 0.0)
                },
                solid(fill, None, radius),
                &children,
            )?;
            ui.fade(node, item_key, theme.hover, Easing::Ease);
            ui.interact(
                node,
                item_key,
                Interact::new().on_click(messages[index].clone()),
            );
            items.push(node);
        }
        if state.focus_ring {
            items.push(focus_ring(ui, theme, radius)?);
        }
        let node = ui.scene().node(
            Style {
                flex_shrink: 0.0,
                ..row(theme, SPACING)
            },
            Visual::Group,
            &items,
        )?;
        ui.anchor(node);
        let (selected, count) = (self.selected, self.options.len());
        ui.interact(
            node,
            key,
            Interact::new().focusable().on_key(move |press| {
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

/// 选中项底下居中的强调色短条，铺满整项的一层里贴底。
fn mark<M>(ui: &mut Ui<M>, theme: &WidgetTheme) -> Result<NodeId, RenderError> {
    let (width, height, bottom) = MARK;
    let height = theme.px(height);
    let bar = ui.scene().node(
        Style {
            size: size(Some(theme.px(width)), Some(height)),
            ..Style::default()
        },
        solid(Some(theme.colors.accent), None, height / 2.0),
        &[],
    )?;
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::FLEX_END,
            align_items: AlignItems::CENTER,
            padding: padding(theme, bottom, 0.0),
            ..overlay()
        },
        Visual::Group,
        &[bar],
    )
}
