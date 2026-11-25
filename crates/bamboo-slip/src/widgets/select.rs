//! 下拉框的收起状态：当前值加箭头，点了产出「打开」的消息，弹出的列表是 [`super::PopupMenu`]。
//! [`Look::Mac`] 照系统设置的弹出按钮：没有框，次要色的字加上下箭头，悬停才垫底色；[`Look::Fluent`] 是 WinUI 的 ComboBox。

use std::hash::Hash;
use std::sync::Arc;

use taffy::{JustifyContent, LengthPercentageAuto, NodeId, Style};

use super::WidgetTheme;
use super::chevron::Chevron;
use super::parts::{DISABLED_OPACITY, centered, focus_ring, over, padding, row, size, solid};
use super::theme::Look;
use crate::animation::Easing;
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::{Interact, Ui};

#[derive(Debug, Clone, Copy)]
pub struct Select<'a> {
    value: &'a str,

    /// 弹出菜单里的全部选项：不画，只给搜索当关键词。
    options: &'a [&'a str],

    /// 最小宽度（点）；值比它长时跟着撑开，不让箭头被挤出框外。`None` 按内容撑开。
    width: Option<f32>,

    /// 最大宽度（点）；值再长就截短补「…」，框不再变宽。`None` 不设上限。
    max_width: Option<f32>,

    disabled: bool,
}

impl<'a> Select<'a> {
    pub fn new(value: &'a str) -> Self {
        Self {
            value,
            options: &[],
            width: None,
            max_width: None,
            disabled: false,
        }
    }

    /// 弹出菜单里的全部选项，搜索时按它们也能找到这一行。
    pub fn options(self, options: &'a [&'a str]) -> Self {
        Self { options, ..self }
    }

    pub fn width(self, points: f32) -> Self {
        Self {
            width: Some(points),
            ..self
        }
    }

    pub fn max_width(self, points: f32) -> Self {
        Self {
            max_width: Some(points),
            ..self
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
        on_open: M,
    ) -> Result<NodeId, RenderError> {
        let key = ui.key(local);
        let state = ui.state(key);
        let colors = &theme.colors;
        let radius = theme.px(theme.radius);
        let mac = theme.look == Look::Mac;
        let active = !self.disabled;
        let (text_size, text_color) = match theme.look {
            Look::Plain => (theme.font.size - 1.0, colors.text),
            Look::Mac if active && state.hovered => (theme.font.size, colors.text),
            Look::Mac => (theme.font.size, colors.secondary),
            Look::Fluent => (theme.font.size, colors.text),
        };
        // 值可以被压窄（最小宽 0），压窄了画的时候截短补「…」；箭头不让压
        let value = ui.scene().node(
            Style {
                flex_shrink: 1.0,
                min_size: bound(Some(0.0)),
                ..Style::default()
            },
            Visual::Text {
                text: self.value.to_owned(),
                style: theme.text(text_size, 400, text_color).truncated(),
            },
            &[],
        )?;
        let (chevron, arrow_size) = match theme.look {
            Look::Mac => (Chevron::UpDown, theme.px(12.0)),
            Look::Fluent => (Chevron::Down, theme.px(12.0)),
            Look::Plain => (Chevron::Down, theme.px(10.0)),
        };
        let arrow = ui.scene().node(
            centered(arrow_size, arrow_size),
            Visual::Icon {
                icon: Arc::new(chevron),
                size: arrow_size,
                color: if mac { text_color } else { colors.secondary },
            },
            &[],
        )?;
        let mut children = vec![value, arrow];
        if state.focus_ring {
            children.push(focus_ring(ui, theme, radius)?);
        }
        let base = if theme.look == Look::Fluent {
            colors.button
        } else {
            colors.surface
        };
        let fill = if active && state.pressed {
            over(base, colors.pressed)
        } else if active && state.hovered {
            over(base, colors.hover)
        } else {
            base
        };
        let node = if mac {
            // 没有框：悬停才垫底色；右边往外让出留白，字的右缘与行里别的控件对齐
            let fill = (active && (state.hovered || state.pressed)).then_some(if state.pressed {
                colors.pressed
            } else {
                colors.hover
            });
            ui.scene().node(
                Style {
                    padding: taffy::Rect {
                        left: taffy::LengthPercentage::length(theme.px(8.0)),
                        right: taffy::LengthPercentage::length(theme.px(5.0)),
                        top: taffy::LengthPercentage::length(0.0),
                        bottom: taffy::LengthPercentage::length(0.0),
                    },
                    margin: taffy::Rect {
                        right: LengthPercentageAuto::length(-theme.px(5.0)),
                        ..taffy::Rect::zero()
                    },
                    size: size(None, Some(theme.px(24.0))),
                    max_size: bound(self.max_width.map(|width| theme.px(width))),
                    flex_shrink: 0.0,
                    ..row(theme, 4.0)
                },
                solid(fill, None, radius),
                &children,
            )?
        } else {
            let (side, min_width) = match theme.look {
                Look::Fluent => (11.0, self.width.map(|width| width.max(160.0))),
                _ => (10.0, self.width),
            };
            ui.scene().node(
                Style {
                    padding: padding(theme, 0.0, side),
                    size: size(None, Some(theme.px(theme.control_height))),
                    min_size: bound(min_width.map(|width| theme.px(width))),
                    max_size: bound(self.max_width.map(|width| theme.px(width))),
                    justify_content: JustifyContent::SPACE_BETWEEN,
                    flex_shrink: 0.0,
                    ..row(theme, 12.0)
                },
                solid(Some(fill), Some((theme.px(1.0), colors.line)), radius),
                &children,
            )?
        };
        ui.fade(node, key, theme.hover, Easing::Ease);
        if self.disabled {
            ui.scene().set_opacity(node, DISABLED_OPACITY);
        }
        ui.interact(
            node,
            key,
            Interact::new()
                .on_click(on_open)
                .focusable()
                .disabled(self.disabled),
        );
        ui.keyword(node, self.value);
        for option in self.options {
            ui.keyword(node, option);
        }
        Ok(node)
    }
}

/// 只限宽度的 `min_size` / `max_size`（像素）；`None` 不限。
fn bound(width: Option<f32>) -> taffy::Size<LengthPercentageAuto> {
    taffy::Size {
        width: width.map_or(LengthPercentageAuto::auto(), LengthPercentageAuto::length),
        height: LengthPercentageAuto::auto(),
    }
}
