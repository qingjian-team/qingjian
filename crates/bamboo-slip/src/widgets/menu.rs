//! 弹出菜单：贴在触发它的控件（下拉框）下面（下面放不下时上面）的一列选项，长了能滚；选中项打勾，悬停垫底色，
//! 暂时不能选的项变灰、点了没反应。
//! 画在同一个窗口里（不能超出窗口），盖一层透明的遮挡接住菜单外的点击，点外面就关。返回的节点要放在根节点的最后一个子节点，才盖得住别的东西。

use std::hash::Hash;
use std::sync::Arc;

use taffy::{Dimension, FlexDirection, LengthPercentageAuto, NodeId, Position, Rect, Size, Style};

use super::ScrollArea;
use super::WidgetTheme;
use super::check_mark::CheckMark;
use super::parts::{DISABLED_OPACITY, centered, label, overlay, padding};
use super::popup_menu::{item_style, item_visual, panel_shadow, panel_visual};
use crate::animation::Easing;
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::{Interact, Ui};

/// 菜单最宽、最高（点），离窗口底边与触发控件的距离（点）。
const MIN_WIDTH: f32 = 200.0;

const MAX_HEIGHT: f32 = 320.0;

const MARGIN: f32 = 12.0;

const GAP: f32 = 4.0;

pub struct Menu<'a> {
    items: &'a [&'a str],

    selected: Option<usize>,

    /// 变灰的项（序号）。
    disabled: &'a [usize],

    /// 触发控件的矩形（像素，上一帧的）与窗口大小（像素）。
    anchor: (f32, f32, f32, f32),

    window: (f32, f32),
}

impl<'a> Menu<'a> {
    /// `anchor` 用 [`Ui::rect`] 取触发控件上一帧的位置；`window` 是根节点（窗口）的大小。
    pub fn new(
        items: &'a [&'a str],
        selected: Option<usize>,
        anchor: (f32, f32, f32, f32),
        window: (f32, f32),
    ) -> Self {
        Self {
            items,
            selected,
            disabled: &[],
            anchor,
            window,
        }
    }

    pub fn disabled(self, disabled: &'a [usize]) -> Self {
        Self { disabled, ..self }
    }

    /// `on_pick(序号)` 点了一项时产出；`on_dismiss` 点了菜单外面时产出（Esc 由使用方在根上接）。
    pub fn build<M: Clone + 'static>(
        self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        local: impl Hash,
        on_pick: impl Fn(usize) -> M,
        on_dismiss: M,
    ) -> Result<NodeId, RenderError> {
        let key = ui.key(local);
        let colors = &theme.colors;
        let radius = theme.px(theme.radius);
        let (x, y, width, height) = self.anchor;
        let (gap, margin) = (theme.px(GAP), theme.px(MARGIN));
        // 下面放不下（连一半的最大高度都不够）而上面更宽敞时往上开，同原生菜单
        let below = self.window.1 - (y + height + gap) - margin;
        let above = y - gap - margin;
        let upward = below < theme.px(MAX_HEIGHT) / 2.0 && above > below;
        let max_height = theme
            .px(MAX_HEIGHT)
            .min(if upward { above } else { below })
            .max(theme.px(80.0));
        let (top, bottom) = if upward {
            (
                LengthPercentageAuto::auto(),
                LengthPercentageAuto::length(self.window.1 - (y - gap)),
            )
        } else {
            (
                LengthPercentageAuto::length(y + height + gap),
                LengthPercentageAuto::auto(),
            )
        };
        let width = width.max(theme.px(MIN_WIDTH));
        let left = x.min(self.window.0 - width - margin).max(0.0);

        let mut rows = Vec::with_capacity(self.items.len());
        for (index, item) in self.items.iter().enumerate() {
            let item_key = key.child(index);
            let state = ui.state(item_key);
            let side = theme.px(14.0);
            let mut mark = Vec::new();
            if self.selected == Some(index) {
                mark.push(ui.scene().node(
                    centered(side, side),
                    Visual::Icon {
                        icon: Arc::new(CheckMark),
                        size: side,
                        color: colors.accent,
                    },
                    &[],
                )?);
            }
            let check = ui
                .scene()
                .node(centered(side, side), Visual::Group, &mark)?;
            let text = label(ui, theme, item, theme.font.size, 400, colors.text)?;
            let disabled = self.disabled.contains(&index);
            let fill =
                (!disabled && (state.hovered || state.pressed)).then_some(if state.pressed {
                    colors.pressed
                } else {
                    colors.hover
                });
            let node =
                ui.scene()
                    .node(item_style(theme), item_visual(theme, fill), &[check, text])?;
            if disabled {
                ui.scene().set_opacity(node, DISABLED_OPACITY);
            }
            ui.fade(node, item_key, theme.hover, Easing::Ease);
            ui.interact(
                node,
                item_key,
                Interact::new().on_click(on_pick(index)).disabled(disabled),
            );
            rows.push(node);
        }
        let list = ui.scene().node(
            Style {
                flex_direction: FlexDirection::Column,
                padding: padding(theme, 4.0, 4.0),
                ..Style::default()
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
        let panel = ui.scene().node(
            Style {
                position: Position::Absolute,
                size: Size {
                    width: Dimension::length(width),
                    height: Dimension::auto(),
                },
                inset: Rect {
                    left: LengthPercentageAuto::length(left),
                    top,
                    right: LengthPercentageAuto::auto(),
                    bottom,
                },
                flex_direction: FlexDirection::Column,
                ..Style::default()
            },
            panel_visual(theme),
            &[scroll],
        )?;
        ui.scene().set_clip(panel, radius);
        ui.scene().set_effects(panel, vec![panel_shadow(theme)]);
        let scrim = ui.scene().node(overlay(), Visual::Group, &[])?;
        ui.interact(
            scrim,
            key.child("outside"),
            Interact::new().on_click(on_dismiss),
        );
        ui.scene().node(overlay(), Visual::Group, &[scrim, panel])
    }
}
