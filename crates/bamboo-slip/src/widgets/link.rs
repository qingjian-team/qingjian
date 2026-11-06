//! 文字链接：一段强调色的字，悬停加下划线，点了产出消息（打开网页由使用方做）；能拿焦点，回车 / 空格同点击。

use std::hash::Hash;

use taffy::{NodeId, Style};

use super::WidgetTheme;
use super::parts::focus_ring;
use crate::animation::Easing;
use crate::error::RenderError;
use crate::scene::Visual;
use crate::text::Decoration;
use crate::ui::{Cursor, Interact, Ui};

#[derive(Debug, Clone, Copy)]
pub struct Link<'a> {
    text: &'a str,

    /// 字号（点）；不给用正文字号。
    points: Option<f32>,
}

impl<'a> Link<'a> {
    pub fn new(text: &'a str) -> Self {
        Self { text, points: None }
    }

    /// 放进小字里时跟着用小字号。
    pub fn points(self, points: f32) -> Self {
        Self {
            points: Some(points),
            ..self
        }
    }

    pub fn build<M: Clone + 'static>(
        self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        local: impl Hash,
        on_click: M,
    ) -> Result<NodeId, RenderError> {
        let key = ui.key(local);
        let state = ui.state(key);
        let mut style = theme.text(
            self.points.unwrap_or(theme.font.size),
            400,
            theme.colors.accent,
        );
        if state.hovered || state.focus_ring {
            style = style.decorated(Decoration::Underline);
        }
        let text = ui.scene().node(
            Style::default(),
            Visual::Text {
                text: self.text.to_owned(),
                style,
            },
            &[],
        )?;
        let mut children = vec![text];
        if state.focus_ring {
            children.push(focus_ring(ui, theme, theme.px(2.0))?);
        }
        let node = ui.scene().node(
            Style {
                flex_shrink: 0.0,
                ..Style::default()
            },
            Visual::Group,
            &children,
        )?;
        ui.fade(node, key, theme.hover, Easing::Ease);
        ui.interact(
            node,
            key,
            Interact::new()
                .on_click(on_click)
                .focusable()
                .cursor(Cursor::Pointer),
        );
        Ok(node)
    }
}
