//! 设置页的分组：分组标题，和把几行设置装在一起的卡片（画法随 [`Look`]：一张描边卡片、圆角分组框或一行一张卡片）。

use taffy::NodeId;

use super::WidgetTheme;
use super::parts::{column, label, padding, size, solid};
use super::setting_row::spotlight;
use super::theme::Look;
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::{Interact, Ui};

/// 分组标题：[`Look::Plain`] 是次要色小字，系统设置的两种外观是正文色的粗体；略向里缩，与卡片里的文字对齐。
pub fn section_title<M>(
    ui: &mut Ui<M>,
    theme: &WidgetTheme,
    text: &str,
) -> Result<NodeId, RenderError> {
    let key = ui.key(("section", text));
    ui.searchable(key, (text, None), &[], &[]);
    let spot = spotlight(ui, theme, key, (5.0, 4.0))?;
    let (size, color, inset, top) = match theme.look {
        Look::Plain => (theme.small + 1.0, theme.colors.secondary, 4.0, 8.0),
        Look::Mac => (theme.font.size, theme.colors.text, 4.0, 10.0),
        Look::Fluent => (theme.font.size, theme.colors.text, 2.0, 18.0),
    };
    let title = label(ui, theme, text, size, 600, color)?;
    let node = ui.scene().node(
        taffy::Style {
            padding: padding(theme, 0.0, inset),
            margin: taffy::Rect {
                top: taffy::LengthPercentageAuto::length(theme.px(top)),
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

/// 卡片：`rows` 竖着排，相邻两行之间一条分隔线；[`Look::Fluent`] 改成一行一张卡片、卡片间留缝。
pub fn group<M>(
    ui: &mut Ui<M>,
    theme: &WidgetTheme,
    rows: &[NodeId],
) -> Result<NodeId, RenderError> {
    let colors = &theme.colors;
    if theme.look == Look::Fluent {
        let mut cards = Vec::with_capacity(rows.len());
        for &row in rows {
            cards.push(ui.scene().node(
                taffy::Style {
                    padding: padding(theme, 0.0, 16.0),
                    ..column(theme, 0.0)
                },
                solid(
                    Some(colors.surface),
                    Some((theme.px(1.0), colors.line)),
                    theme.px(theme.radius * 1.5),
                ),
                &[row],
            )?);
        }
        return ui.scene().node(column(theme, 4.0), Visual::Group, &cards);
    }
    // 系统设置的分组框只描半个点的边、圆角 10，行间分隔线缩在左右留白里
    let (inset, border, radius) = match theme.look {
        Look::Mac => (14.0, 0.5, 10.0),
        _ => (16.0, 1.0, theme.radius * 1.5),
    };
    let mut children = Vec::with_capacity(rows.len() * 2);
    for (index, &row) in rows.iter().enumerate() {
        if index > 0 {
            children.push(ui.scene().node(
                taffy::Style {
                    size: size(None, Some(theme.px(border).max(1.0))),
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
            padding: padding(theme, 0.0, inset),
            ..column(theme, 0.0)
        },
        solid(
            Some(colors.surface),
            Some((theme.px(border), colors.line)),
            theme.px(radius),
        ),
        &children,
    )
}
