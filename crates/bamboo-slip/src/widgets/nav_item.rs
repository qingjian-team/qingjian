//! 导航项：侧栏里的一页，图标加名字。选中项垫一块底，换页时这块底滑到新的一项上（同一作用域里的导航项共用它的 key）。

use std::hash::Hash;
use std::sync::Arc;

use taffy::{NodeId, Style};

use super::WidgetTheme;
use super::parts::{centered, focus_ring, label, overlay, padding, row, solid};
use crate::animation::Easing;
use crate::error::RenderError;
use crate::scene::{Icon, Visual};
use crate::ui::{Interact, Ui};

/// 图标边长（点）。
const ICON: f32 = 16.0;

pub struct NavItem<'a> {
    label: &'a str,

    icon: Option<Arc<dyn Icon>>,

    selected: bool,

    /// 选中时额外叠上的装饰（使用方建好的节点，如品牌的选中标记），绝对定位相对这一项。
    accessory: Option<NodeId>,
}

impl<'a> NavItem<'a> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            icon: None,
            selected: false,
            accessory: None,
        }
    }

    pub fn icon(self, icon: Arc<dyn Icon>) -> Self {
        Self {
            icon: Some(icon),
            ..self
        }
    }

    pub fn selected(self, selected: bool) -> Self {
        Self { selected, ..self }
    }

    pub fn accessory(self, node: NodeId) -> Self {
        Self {
            accessory: Some(node),
            ..self
        }
    }

    pub fn build<M: Clone + 'static>(
        self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        local: impl Hash,
        on_select: M,
    ) -> Result<NodeId, RenderError> {
        let key = ui.key(local);
        let selection_key = ui.key("selection");
        let state = ui.state(key);
        let colors = &theme.colors;
        let radius = theme.px(theme.radius);
        let color = if self.selected {
            colors.selected_text
        } else {
            colors.text
        };

        let mut children = Vec::new();
        if self.selected {
            let selection =
                ui.scene()
                    .node(overlay(), solid(Some(colors.selected), None, radius), &[])?;
            ui.transition(selection, selection_key, theme.flip, theme.flip_easing);
            children.push(selection);
        }
        if let Some(icon) = self.icon {
            let side = theme.px(ICON);
            children.push(ui.scene().node(
                centered(side, side),
                Visual::Icon {
                    icon,
                    size: side,
                    color,
                },
                &[],
            )?);
        }
        let weight = if self.selected { 600 } else { 400 };
        children.push(label(
            ui,
            theme,
            self.label,
            theme.font.size,
            weight,
            color,
        )?);
        if self.selected
            && let Some(accessory) = self.accessory
        {
            children.push(accessory);
        }
        if state.focus_ring {
            children.push(focus_ring(ui, theme, radius)?);
        }
        let fill =
            (!self.selected && (state.hovered || state.pressed)).then_some(if state.pressed {
                colors.pressed
            } else {
                colors.hover
            });
        let node = ui.scene().node(
            Style {
                padding: padding(theme, 6.5, 8.0),
                ..row(theme, 10.0)
            },
            solid(fill, None, radius),
            &children,
        )?;
        ui.fade(node, key, theme.hover, Easing::Ease);
        ui.interact(node, key, Interact::new().on_click(on_select).focusable());
        Ok(node)
    }
}
