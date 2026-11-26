//! 弹出菜单：贴在触发它的控件（下拉框）下面（下面放不下时上面）的一列选项，长了能滚；选中项打勾，悬停垫底色，
//! 暂时不能选的项变灰、点了没反应。
//! 画在同一个窗口里（不能超出窗口），盖一层透明的遮挡接住菜单外的点击，点外面就关。返回的节点要放在根节点的最后一个子节点，才盖得住别的东西。
//! [`Look::Mac`] 与 [`Look::Fluent`] 照两边系统的弹出菜单：有选中项时它盖在触发控件上，再收进窗口；
//! Mac 悬停项是强调色底白字、选中项前打勾，Fluent 选中项垫浅底、左边一段强调色竖条。

use std::hash::Hash;
use std::sync::Arc;

use taffy::{Dimension, FlexDirection, LengthPercentageAuto, NodeId, Position, Rect, Size, Style};

use super::ScrollArea;
use super::WidgetTheme;
use super::check_mark::CheckMark;
use super::parts::{DISABLED_OPACITY, centered, label, overlay, padding, row, solid};
use super::popup_menu::{item_style, item_visual, panel_shadow, panel_visual};
use super::theme::Look;
use crate::animation::Easing;
use crate::error::RenderError;
use crate::scene::{Icon, Visual};
use crate::ui::{Interact, Ui};

/// 菜单最宽、最高（点），离窗口底边与触发控件的距离（点）。
const MIN_WIDTH: f32 = 200.0;

const MAX_HEIGHT: f32 = 320.0;

const MARGIN: f32 = 12.0;

const GAP: f32 = 4.0;

/// 照系统设置画的两种菜单一项占多高（点）：Mac 24，Fluent 32 加上下各 2 的间隔。
const MAC_ITEM: f32 = 24.0;

const FLUENT_ITEM: f32 = 36.0;

pub struct Menu<'a> {
    items: &'a [&'a str],

    selected: Option<usize>,

    /// 变灰的项（序号）。
    disabled: &'a [usize],

    /// 各项前面的图标（按序号），占打勾那一格。
    icons: &'a [Arc<dyn Icon>],

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
            icons: &[],
            anchor,
            window,
        }
    }

    pub fn disabled(self, disabled: &'a [usize]) -> Self {
        Self { disabled, ..self }
    }

    pub fn icons(self, icons: &'a [Arc<dyn Icon>]) -> Self {
        Self { icons, ..self }
    }

    /// 选中项盖在触发控件上的摆法，收进窗口：返回上、下边距，最高多高，宽，左边距（像素）。
    fn cover(
        &self,
        theme: &WidgetTheme,
        selected: usize,
    ) -> (LengthPercentageAuto, LengthPercentageAuto, f32, f32, f32) {
        let (x, y, width, height) = self.anchor;
        let look = theme.look;
        let item = theme.px(if look == Look::Mac {
            MAC_ITEM
        } else {
            FLUENT_ITEM
        });
        let inner = theme.px(menu_padding(look));
        let margin = theme.px(8.0);
        let full = self.items.len() as f32 * item + inner * 2.0;
        let max_height = full.min(self.window.1 - margin * 2.0);
        let top = y + height / 2.0 - (inner + selected as f32 * item + item / 2.0);
        let top = top.min(self.window.1 - margin - max_height).max(margin);
        // 菜单项的字与触发控件上的字左缘对齐：Mac 的字前面留着打勾那一格
        let (min_width, shift) = match look {
            Look::Mac => (150.0, 19.0),
            _ => (160.0, inner / theme.scale),
        };
        let width = (width + inner * 2.0).max(theme.px(min_width));
        let left = (x - theme.px(shift))
            .min(self.window.0 - margin - width)
            .max(margin);
        (
            LengthPercentageAuto::length(top),
            LengthPercentageAuto::auto(),
            max_height,
            width,
            left,
        )
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
        let radius = theme.px(theme.popup_radius);
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
        let look = theme.look;
        // 没有选中项的（右键菜单）照旧从触发点往下开
        let cover = self.selected.filter(|_| look != Look::Plain);
        let (top, bottom, max_height, width, left) = if let Some(selected) = cover {
            self.cover(theme, selected)
        } else {
            let width = width.max(theme.px(MIN_WIDTH));
            let left = x.min(self.window.0 - width - margin).max(0.0);
            (top, bottom, max_height, width, left)
        };

        let mut rows = Vec::with_capacity(self.items.len());
        for (index, item) in self.items.iter().enumerate() {
            let item_key = key.child(index);
            let state = ui.state(item_key);
            let side = theme.px(if look == Look::Mac { 12.0 } else { 14.0 });
            let disabled = self.disabled.contains(&index);
            let selected = self.selected == Some(index);
            let hot = !disabled && (state.hovered || state.pressed);
            let ink = if look == Look::Mac && hot {
                colors.on_accent
            } else {
                colors.text
            };
            let mut mark = Vec::new();
            if let Some(icon) = self.icons.get(index) {
                mark.push(ui.scene().node(
                    centered(side, side),
                    Visual::Icon {
                        icon: icon.clone(),
                        size: side,
                        color: ink,
                    },
                    &[],
                )?);
            } else if selected && look != Look::Fluent {
                mark.push(ui.scene().node(
                    centered(side, side),
                    Visual::Icon {
                        icon: Arc::new(CheckMark),
                        size: side,
                        color: if look == Look::Mac {
                            ink
                        } else {
                            colors.accent
                        },
                    },
                    &[],
                )?);
            }
            let text = label(ui, theme, item, theme.font.size, 400, ink)?;
            let node = match look {
                Look::Plain => {
                    let check = ui
                        .scene()
                        .node(centered(side, side), Visual::Group, &mark)?;
                    let fill = hot.then_some(if state.pressed {
                        colors.pressed
                    } else {
                        colors.hover
                    });
                    ui.scene()
                        .node(item_style(theme), item_visual(theme, fill), &[check, text])?
                }
                Look::Mac => {
                    let check = ui
                        .scene()
                        .node(centered(side, side), Visual::Group, &mark)?;
                    let fill = hot.then_some(colors.accent);
                    ui.scene().node(
                        Style {
                            size: Size {
                                width: Dimension::auto(),
                                height: Dimension::length(theme.px(MAC_ITEM)),
                            },
                            padding: Rect {
                                left: taffy::LengthPercentage::length(theme.px(6.0)),
                                right: taffy::LengthPercentage::length(theme.px(12.0)),
                                top: taffy::LengthPercentage::length(0.0),
                                bottom: taffy::LengthPercentage::length(0.0),
                            },
                            flex_shrink: 0.0,
                            ..row(theme, 4.0)
                        },
                        solid(fill, None, theme.px(5.0)),
                        &[check, text],
                    )?
                }
                Look::Fluent => {
                    let mut children = mark;
                    if selected {
                        children.push(pill(ui, theme)?);
                    }
                    children.push(text);
                    let fill = (selected || hot).then_some(if state.pressed {
                        colors.pressed
                    } else {
                        colors.selected
                    });
                    ui.scene().node(
                        Style {
                            size: Size {
                                width: Dimension::auto(),
                                height: Dimension::length(theme.px(FLUENT_ITEM - 4.0)),
                            },
                            margin: Rect {
                                top: LengthPercentageAuto::length(theme.px(2.0)),
                                bottom: LengthPercentageAuto::length(theme.px(2.0)),
                                left: LengthPercentageAuto::length(0.0),
                                right: LengthPercentageAuto::length(0.0),
                            },
                            padding: padding(theme, 0.0, 11.0),
                            flex_shrink: 0.0,
                            ..row(theme, 8.0)
                        },
                        solid(fill, None, theme.px(4.0)),
                        &children,
                    )?
                }
            };
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
        let inner = menu_padding(look);
        let list = ui.scene().node(
            Style {
                flex_direction: FlexDirection::Column,
                padding: padding(theme, inner, inner),
                ..Style::default()
            },
            Visual::Group,
            &rows,
        )?;
        // 刚打开（上一帧还没有这个菜单）时把选中项滚进视野：太长的菜单不然只露出前面几项
        if ui.rect(key.child("outside")).is_none()
            && let Some(selected) = self.selected
        {
            ui.reveal(key.child(selected));
        }
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
            panel_visual(theme, theme.px(theme.popup_radius), theme.px(1.0)),
            &[scroll],
        )?;
        ui.scene().set_clip(panel, radius);
        ui.scene().set_effects(panel, vec![panel_shadow(theme)]);
        if !theme.popup_motion.is_zero() {
            let offset = match look {
                Look::Plain => theme.px(if upward { GAP * 2.0 } else { -GAP * 2.0 }),
                Look::Mac => 0.0,
                Look::Fluent => -theme.px(GAP * 2.0),
            };
            ui.enter(
                panel,
                key.child("panel"),
                theme.popup_motion,
                Easing::Bezier([0.0, 0.0, 0.0, 1.0]),
                offset,
            );
        }
        let scrim = ui.scene().node(overlay(), Visual::Group, &[])?;
        ui.interact(
            scrim,
            key.child("outside"),
            Interact::new().on_click(on_dismiss),
        );
        ui.scene().node(overlay(), Visual::Group, &[scrim, panel])
    }
}

/// 菜单卡片的内边距（点）。
fn menu_padding(look: Look) -> f32 {
    if look == Look::Mac { 5.0 } else { 4.0 }
}

/// Fluent 选中项左边的强调色竖条：3 点宽、16 点高，上下居中。
fn pill<M>(ui: &mut Ui<M>, theme: &WidgetTheme) -> Result<NodeId, RenderError> {
    let (width, height) = (theme.px(3.0), theme.px(16.0));
    let bar = ui.scene().node(
        Style {
            size: Size {
                width: Dimension::length(width),
                height: Dimension::length(height),
            },
            ..Style::default()
        },
        solid(Some(theme.colors.accent), None, width / 2.0),
        &[],
    )?;
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            justify_content: taffy::JustifyContent::CENTER,
            position: Position::Absolute,
            inset: Rect {
                left: LengthPercentageAuto::length(0.0),
                top: LengthPercentageAuto::length(0.0),
                bottom: LengthPercentageAuto::length(0.0),
                right: LengthPercentageAuto::auto(),
            },
            ..Style::default()
        },
        Visual::Group,
        &[bar],
    )
}
