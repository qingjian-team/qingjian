//! 下拉框的收起状态：当前值加朝下的箭头，点了产出「打开」的消息；弹出的选项列表等弹出层做好后补上。

use std::hash::Hash;
use std::sync::Arc;

use taffy::{JustifyContent, NodeId, Style};

use super::WidgetTheme;
use super::chevron::Chevron;
use super::parts::{
    DISABLED_OPACITY, centered, focus_ring, label, over, padding, row, size, solid,
};
use crate::animation::Easing;
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::{Interact, Ui};

#[derive(Debug, Clone, Copy)]
pub struct Select<'a> {
    value: &'a str,

    /// 弹出菜单里的全部选项：不画，只给搜索当关键词。
    options: &'a [&'a str],

    /// 宽度（点）；`None` 按内容撑开。
    width: Option<f32>,

    disabled: bool,
}

impl<'a> Select<'a> {
    pub fn new(value: &'a str) -> Self {
        Self {
            value,
            options: &[],
            width: None,
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
        let value = label(
            ui,
            theme,
            self.value,
            theme.font.size - 1.0,
            400,
            colors.text,
        )?;
        let arrow_size = theme.px(10.0);
        let arrow = ui.scene().node(
            centered(arrow_size, arrow_size),
            Visual::Icon {
                icon: Arc::new(Chevron),
                size: arrow_size,
                color: colors.secondary,
            },
            &[],
        )?;
        let mut children = vec![value, arrow];
        if state.focus_ring {
            children.push(focus_ring(ui, theme, radius)?);
        }
        let active = !self.disabled;
        let fill = if active && state.pressed {
            over(colors.surface, colors.pressed)
        } else if active && state.hovered {
            over(colors.surface, colors.hover)
        } else {
            colors.surface
        };
        let node = ui.scene().node(
            Style {
                padding: padding(theme, 0.0, 10.0),
                size: size(
                    self.width.map(|width| theme.px(width)),
                    Some(theme.px(theme.control_height)),
                ),
                justify_content: Some(JustifyContent::SPACE_BETWEEN),
                flex_shrink: 0.0,
                ..row(theme, 12.0)
            },
            solid(Some(fill), Some((theme.px(1.0), colors.line)), radius),
            &children,
        )?;
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
