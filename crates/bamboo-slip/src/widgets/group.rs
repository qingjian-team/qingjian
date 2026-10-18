//! 设置页的分组：小字的分组标题，和把几行设置装在一起、行间画分隔线的卡片。

use taffy::NodeId;

use super::WidgetTheme;
use super::parts::{column, label, padding, size, solid};
use super::setting_row::spotlight;
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::{Interact, Ui};

/// 分组标题：小字、次要色，略向里缩，与卡片里的文字对齐。
pub fn section_title<M>(
    ui: &mut Ui<M>,
    theme: &WidgetTheme,
    text: &str,
) -> Result<NodeId, RenderError> {
    let key = ui.key(("section", text));
    ui.searchable(key, (text, None), &[], &[]);
    let spot = spotlight(ui, theme, key, (5.0, 4.0))?;
    let title = label(
        ui,
        theme,
        text,
        theme.small + 1.0,
        600,
        theme.colors.secondary,
    )?;
    let node = ui.scene().node(
        taffy::Style {
            padding: padding(theme, 0.0, 4.0),
            margin: taffy::Rect {
                top: taffy::LengthPercentageAuto::length(theme.px(8.0)),
                ..taffy::Rect::zero()
            },
            ..taffy::Style::default()
        },
        Visual::Group,
        &[spot, title],
    )?;
    ui.interact(node, key, Interact::new());
    Ok(node)
}

/// 卡片：`rows` 竖着排，相邻两行之间一条分隔线。
pub fn group<M>(
    ui: &mut Ui<M>,
    theme: &WidgetTheme,
    rows: &[NodeId],
) -> Result<NodeId, RenderError> {
    let colors = &theme.colors;
    let mut children = Vec::with_capacity(rows.len() * 2);
    for (index, &row) in rows.iter().enumerate() {
        if index > 0 {
            children.push(ui.scene().node(
                taffy::Style {
                    size: size(None, Some(theme.px(1.0))),
                    ..taffy::Style::default()
                },
                Visual::solid(colors.line, 0.0),
                &[],
            )?);
        }
        children.push(row);
    }
    ui.scene().node(
        taffy::Style {
            padding: padding(theme, 0.0, 16.0),
            ..column(theme, 0.0)
        },
        solid(
            Some(colors.surface),
            Some((theme.px(1.0), colors.line)),
            theme.px(theme.radius * 1.5),
        ),
        &children,
    )
}
