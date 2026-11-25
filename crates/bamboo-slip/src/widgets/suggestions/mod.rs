//! 建议列表（WinUI 的 AutoSuggestBox 那种）：贴在输入框下面的浮层，一条一行（标题、下面一行小字、右边一段小字），
//! 太长时滚动。高亮的一条（键盘 ↑ ↓ 选到的）垫底色并滚进视野；按键怎么移高亮、回车选哪条由使用方管。
//! 建好的节点自己按根坐标绝对定位，交给 [`Ui::overlay`] 挂到根上。

mod suggestion;

pub use suggestion::Suggestion;

use std::hash::Hash;

use taffy::{Dimension, FlexDirection, LengthPercentageAuto, NodeId, Position, Rect, Size, Style};

use super::WidgetTheme;
use super::parts::{column, padding, row, solid, truncated};
use super::popup_menu::{panel_shadow, panel_visual};
use super::scroll_area::ScrollArea;
use crate::animation::Easing;
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::{Interact, Ui};

/// 浮层离输入框的距离（点）。
const GAP: f32 = 2.0;

/// 浮层最高多少（点），再多就滚动。
const MAX_HEIGHT: f32 = 440.0;

/// 离窗口下边至少留多少（点）。
const MARGIN: f32 = 8.0;

/// 一条的最低高度（点）。
const ITEM: f32 = 36.0;

pub struct Suggestions<'a> {
    items: &'a [Suggestion<'a>],

    /// 输入框的位置与大小（根坐标，像素）：浮层与它左右对齐，贴在它下面。
    anchor: (f32, f32, f32, f32),

    /// 窗口大小（像素），浮层不超出窗口下边。
    window: (f32, f32),

    hot: Option<usize>,

    /// 一条都没有时显示的字。
    empty: &'a str,
}

impl<'a> Suggestions<'a> {
    pub fn new(
        items: &'a [Suggestion<'a>],
        anchor: (f32, f32, f32, f32),
        window: (f32, f32),
    ) -> Self {
        Self {
            items,
            anchor,
            window,
            hot: None,
            empty: "",
        }
    }

    /// 高亮第几条；`None` 是还没用键盘选过。
    pub fn hot(self, hot: Option<usize>) -> Self {
        Self { hot, ..self }
    }

    pub fn empty(self, empty: &'a str) -> Self {
        Self { empty, ..self }
    }

    pub fn build<M: Clone + 'static>(
        self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        local: impl Hash,
        on_pick: impl Fn(usize) -> M,
    ) -> Result<NodeId, RenderError> {
        let key = ui.key(local);
        let colors = &theme.colors;
        let (x, y, width, height) = self.anchor;
        let top = y + height + theme.px(GAP);
        let max_height = theme
            .px(MAX_HEIGHT)
            .min(self.window.1 - top - theme.px(MARGIN))
            .max(theme.px(ITEM));
        let mut rows = Vec::with_capacity(self.items.len().max(1));
        for (index, item) in self.items.iter().enumerate() {
            let item_key = key.child(index);
            let state = ui.state(item_key);
            let fill = if state.pressed {
                Some(colors.pressed)
            } else if state.hovered || self.hot == Some(index) {
                Some(colors.hover)
            } else {
                None
            };
            let mut texts = vec![truncated(
                ui,
                theme.text(theme.font.size, 400, colors.text).truncated(),
                item.title,
            )?];
            if let Some(detail) = item.detail {
                texts.push(truncated(
                    ui,
                    theme.text(theme.small, 400, colors.secondary).truncated(),
                    detail,
                )?);
            }
            let texts = ui.scene().node(
                Style {
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    min_size: Size {
                        width: LengthPercentageAuto::length(0.0),
                        height: LengthPercentageAuto::auto(),
                    },
                    ..column(theme, 0.0)
                },
                Visual::Group,
                &texts,
            )?;
            let mut children = vec![texts];
            if let Some(trailing) = item.trailing {
                children.push(ui.scene().node(
                    Style {
                        flex_shrink: 0.0,
                        ..Style::default()
                    },
                    Visual::Text {
                        text: trailing.to_owned(),
                        style: theme.text(theme.small, 400, colors.secondary),
                    },
                    &[],
                )?);
            }
            let node = ui.scene().node(
                Style {
                    min_size: Size {
                        width: LengthPercentageAuto::auto(),
                        height: LengthPercentageAuto::length(theme.px(ITEM)),
                    },
                    margin: Rect {
                        top: LengthPercentageAuto::length(theme.px(2.0)),
                        bottom: LengthPercentageAuto::length(theme.px(2.0)),
                        left: LengthPercentageAuto::length(0.0),
                        right: LengthPercentageAuto::length(0.0),
                    },
                    padding: padding(theme, 4.0, 11.0),
                    flex_shrink: 0.0,
                    ..row(theme, 12.0)
                },
                solid(fill, None, theme.px(theme.radius)),
                &children,
            )?;
            ui.fade(node, item_key, theme.hover, Easing::Ease);
            ui.interact(node, item_key, Interact::new().on_click(on_pick(index)));
            rows.push(node);
        }
        if rows.is_empty() {
            let text = ui.scene().node(
                Style::default(),
                Visual::Text {
                    text: self.empty.to_owned(),
                    style: theme.text(theme.font.size, 400, colors.secondary),
                },
                &[],
            )?;
            rows.push(ui.scene().node(
                Style {
                    padding: padding(theme, 8.0, 11.0),
                    ..row(theme, 0.0)
                },
                Visual::Group,
                &[text],
            )?);
        }
        if let Some(hot) = self.hot {
            ui.reveal(key.child(hot));
        }
        let list = ui.scene().node(
            Style {
                padding: padding(theme, 2.0, 4.0),
                ..column(theme, 0.0)
            },
            Visual::Group,
            &rows,
        )?;
        let scroll = ui.scope(key, |ui| {
            ScrollArea::new(Style {
                max_size: Size {
                    width: LengthPercentageAuto::auto(),
                    height: LengthPercentageAuto::length(max_height),
                },
                ..Style::default()
            })
            .build(ui, theme, "scroll", list)
        })?;
        let radius = theme.px(theme.popup_radius);
        let panel = ui.scene().node(
            Style {
                position: Position::Absolute,
                size: Size {
                    width: Dimension::length(width),
                    height: Dimension::auto(),
                },
                inset: Rect {
                    left: LengthPercentageAuto::length(x),
                    top: LengthPercentageAuto::length(top),
                    right: LengthPercentageAuto::auto(),
                    bottom: LengthPercentageAuto::auto(),
                },
                flex_direction: FlexDirection::Column,
                ..Style::default()
            },
            panel_visual(theme, radius, theme.px(1.0)),
            &[scroll],
        )?;
        ui.scene().set_clip(panel, radius);
        ui.scene().set_effects(panel, vec![panel_shadow(theme)]);
        Ok(panel)
    }
}
