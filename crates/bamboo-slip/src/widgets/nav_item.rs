//! 导航项：侧栏里的一页，图标加名字。选中项垫一块底，换页时这块底滑到新的一项上（同一作用域里的导航项共用它的 key）。
//! [`Look::Mac`] 的选中项是强调色底白字；[`Look::Fluent`] 是浅灰底，左边一段强调色竖条。
//! 名字下面可以再挂几行小字（[`NavItem::line`]，搜索结果的摘录与所在位置），放不下的截短。

use std::hash::Hash;
use std::sync::Arc;

use taffy::{NodeId, Style};

use super::WidgetTheme;
use super::parts::{centered, column, focus_ring, label, overlay, padding, row, solid, truncated};
use super::theme::Look;
use crate::animation::Easing;
use crate::error::RenderError;
use crate::scene::{Icon, Visual};
use crate::ui::{Interact, Ui};

/// 图标边长（点）。
const ICON: f32 = 16.0;

pub struct NavItem<'a> {
    label: &'a str,

    icon: Option<Arc<dyn Icon>>,

    /// 使用方建好的图标节点（彩色方块、彩色 SVG），代替 `icon`，选中时不跟着变色。
    leading: Option<NodeId>,

    selected: bool,

    /// 选中时额外叠上的装饰（使用方建好的节点，如品牌的选中标记），绝对定位相对这一项。
    accessory: Option<NodeId>,

    /// 名字下面的小字，一项一行。
    lines: Vec<&'a str>,
}

impl<'a> NavItem<'a> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            icon: None,
            leading: None,
            selected: false,
            accessory: None,
            lines: Vec::new(),
        }
    }

    /// 名字下面加一行次要色的小字；有小字时名字与小字放不下都截短。
    pub fn line(mut self, text: &'a str) -> Self {
        self.lines.push(text);
        self
    }

    pub fn icon(self, icon: Arc<dyn Icon>) -> Self {
        Self {
            icon: Some(icon),
            ..self
        }
    }

    /// 见 [`Self::leading`] 字段。
    pub fn leading(self, node: NodeId) -> Self {
        Self {
            leading: Some(node),
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
        let (radius, fill, color) = match theme.look {
            Look::Mac => (8.0, colors.accent, colors.on_accent),
            Look::Fluent => (theme.radius, colors.selected, colors.text),
            Look::Plain => (theme.radius, colors.selected, colors.selected_text),
        };
        let radius = theme.px(radius);
        let color = if self.selected { color } else { colors.text };

        let mut children = Vec::new();
        if self.selected {
            let mut marks = Vec::new();
            if theme.look == Look::Fluent {
                marks.push(pill(ui, theme)?);
            }
            let selection = ui
                .scene()
                .node(overlay(), solid(Some(fill), None, radius), &marks)?;
            ui.transition(selection, selection_key, theme.flip, theme.flip_easing);
            children.push(selection);
        }
        if let Some(leading) = self.leading {
            children.push(leading);
        } else if let Some(icon) = self.icon {
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
        let weight = if self.selected && theme.look == Look::Plain {
            600
        } else {
            400
        };
        if self.lines.is_empty() {
            children.push(label(
                ui,
                theme,
                self.label,
                theme.font.size,
                weight,
                color,
            )?);
        } else {
            let secondary = if self.selected && theme.look == Look::Mac {
                colors.on_accent
            } else {
                colors.secondary
            };
            let mut texts = vec![truncated(
                ui,
                theme.text(theme.font.size, weight, color).truncated(),
                self.label,
            )?];
            for line in &self.lines {
                texts.push(truncated(
                    ui,
                    theme.text(theme.small, 400, secondary).truncated(),
                    line,
                )?);
            }
            children.push(ui.scene().node(
                Style {
                    flex_shrink: 1.0,
                    flex_grow: 1.0,
                    min_size: taffy::Size {
                        width: taffy::LengthPercentageAuto::length(0.0),
                        height: taffy::LengthPercentageAuto::auto(),
                    },
                    ..column(theme, 0.0)
                },
                Visual::Group,
                &texts,
            )?);
        }
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
        // 行高：Mac 32（22 点的彩色方块上下各 5），Fluent 36
        let (vertical, horizontal, spacing) = match theme.look {
            Look::Plain => (6.5, 8.0, 10.0),
            Look::Mac => (5.0, 6.0, 8.0),
            Look::Fluent => (8.0, 12.0, 14.0),
        };
        let node = ui.scene().node(
            Style {
                padding: padding(theme, vertical, horizontal),
                ..row(theme, spacing)
            },
            solid(fill, None, radius),
            &children,
        )?;
        ui.fade(node, key, theme.hover, Easing::Ease);
        ui.interact(node, key, Interact::new().on_click(on_select).focusable());
        Ok(node)
    }
}

/// Fluent 选中项左边的强调色竖条：3 点宽、16 点高，上下居中。
fn pill<M>(ui: &mut Ui<M>, theme: &WidgetTheme) -> Result<NodeId, RenderError> {
    let (width, height) = (theme.px(3.0), theme.px(16.0));
    let bar = ui.scene().node(
        Style {
            size: taffy::Size {
                width: taffy::Dimension::length(width),
                height: taffy::Dimension::length(height),
            },
            ..Style::default()
        },
        solid(Some(theme.colors.accent), None, width / 2.0),
        &[],
    )?;
    ui.scene().node(
        Style {
            flex_direction: taffy::FlexDirection::Column,
            justify_content: taffy::JustifyContent::CENTER,
            ..overlay()
        },
        crate::scene::Visual::Group,
        &[bar],
    )
}
