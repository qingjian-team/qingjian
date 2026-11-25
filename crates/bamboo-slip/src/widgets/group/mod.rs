//! 设置页的分组：分组标题，和把几行设置装在一起的卡片（画法随 [`Look`]：一张描边卡片、圆角分组框或一行一张卡片）。
//! 行可以带从属项（[`Branch`]）：竹青照常排、macOS 缩进一级、Windows 收进那一行的展开卡片（SettingsExpander）。

mod branch;

pub use branch::Branch;

use std::sync::Arc;

use taffy::{FlexDirection, JustifyContent, LengthPercentage, NodeId, Rect, Style};

use super::WidgetTheme;
use super::chevron::Chevron;
use super::parts::{centered, column, label, padding, row, size, solid};
use super::setting_row::spotlight;
use super::theme::Look;
use crate::animation::Easing;
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::{Cursor, Interact, Key, Ui};

/// macOS 从属项往里缩多少（点）。
const MAC_INDENT: f32 = 14.0;

/// Windows 展开卡片里从属项左边、右边的留白（点）：左边与带行首图标的标题对齐（16 留白 + 20 图标 + 16 间隔），
/// 右边让出箭头，控件与卡片头一行的控件对齐。
const FLUENT_CHILD_INSET: (f32, f32) = (52.0, 48.0);

/// Windows 展开卡片右边箭头按钮的边长（点）。
const EXPANDER_BUTTON: f32 = 32.0;

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
            cards.push(card(ui, theme, &[row])?);
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

/// 同 [`group`]，行可以带从属项；`on_toggle` 是 Windows 展开卡片的箭头点了发的消息（参数是 [`Branch::expander`] 的 key）。
pub fn group_branches<M: Clone + 'static>(
    ui: &mut Ui<M>,
    theme: &WidgetTheme,
    branches: Vec<Branch>,
    on_toggle: impl Fn(Key) -> M,
) -> Result<NodeId, RenderError> {
    if theme.look != Look::Fluent {
        let mut rows = Vec::new();
        for branch in branches {
            rows.push(branch.head);
            for child in branch.children {
                rows.push(if theme.look == Look::Mac {
                    indented(ui, theme, child)?
                } else {
                    child
                });
            }
        }
        return group(ui, theme, &rows);
    }
    let mut cards = Vec::with_capacity(branches.len());
    for branch in branches {
        let Some((key, open)) = branch.expander else {
            cards.push(card(ui, theme, &[branch.head])?);
            continue;
        };
        cards.push(expander(ui, theme, branch, (key, open), on_toggle(key))?);
    }
    ui.scene().node(column(theme, 4.0), Visual::Group, &cards)
}

/// Windows 的一张卡片：浅底细边、左右留白。
fn card<M>(ui: &mut Ui<M>, theme: &WidgetTheme, rows: &[NodeId]) -> Result<NodeId, RenderError> {
    let colors = &theme.colors;
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
        rows,
    )
}

/// macOS 的从属项：往里缩一级。
fn indented<M>(ui: &mut Ui<M>, theme: &WidgetTheme, row: NodeId) -> Result<NodeId, RenderError> {
    ui.scene().node(
        Style {
            padding: Rect {
                left: LengthPercentage::length(theme.px(MAC_INDENT)),
                ..Rect::zero()
            },
            ..column(theme, 0.0)
        },
        Visual::Group,
        &[row],
    )
}

/// Windows 的展开卡片：头一行右边一个箭头，展开时下面一行一项、上面一条分隔线、垫一层浅底。
fn expander<M: Clone + 'static>(
    ui: &mut Ui<M>,
    theme: &WidgetTheme,
    branch: Branch,
    (key, open): (Key, bool),
    on_toggle: M,
) -> Result<NodeId, RenderError> {
    let colors = &theme.colors;
    let state = ui.state(key);
    let side = theme.px(EXPANDER_BUTTON);
    // 图标从节点左上角画起，节点得跟图标一样大，外面的按钮才摆得正
    let glyph = theme.px(12.0);
    let arrow = ui.scene().node(
        centered(glyph, glyph),
        Visual::Icon {
            icon: Arc::new(if open { Chevron::Up } else { Chevron::Down }),
            size: glyph,
            color: colors.text,
        },
        &[],
    )?;
    let fill = (state.hovered || state.pressed).then_some(if state.pressed {
        colors.pressed
    } else {
        colors.hover
    });
    let button = ui.scene().node(
        Style {
            flex_shrink: 0.0,
            ..centered(side, side)
        },
        solid(fill, None, theme.px(theme.radius)),
        &[arrow],
    )?;
    ui.fade(button, key, theme.hover, Easing::Ease);
    ui.interact(
        button,
        key,
        Interact::new()
            .on_click(on_toggle)
            .focusable()
            .cursor(Cursor::Pointer),
    );
    let head = ui.scene().node(
        Style {
            flex_grow: 1.0,
            flex_shrink: 1.0,
            min_size: taffy::Size {
                width: taffy::LengthPercentageAuto::length(0.0),
                height: taffy::LengthPercentageAuto::auto(),
            },
            ..column(theme, 0.0)
        },
        Visual::Group,
        &[branch.head],
    )?;
    let headline = ui.scene().node(
        Style {
            padding: Rect {
                left: LengthPercentage::length(theme.px(16.0)),
                right: LengthPercentage::length(theme.px(8.0)),
                ..Rect::zero()
            },
            justify_content: JustifyContent::SPACE_BETWEEN,
            ..row(theme, 8.0)
        },
        Visual::Group,
        &[head, button],
    )?;
    let mut children = vec![headline];
    if open {
        let (left, right) = FLUENT_CHILD_INSET;
        for child in branch.children {
            children.push(ui.scene().node(
                Style {
                    size: size(None, Some(theme.px(1.0))),
                    ..Style::default()
                },
                Visual::solid(colors.line, 0.0),
                &[],
            )?);
            children.push(ui.scene().node(
                Style {
                    padding: Rect {
                        left: LengthPercentage::length(theme.px(left)),
                        right: LengthPercentage::length(theme.px(right)),
                        ..Rect::zero()
                    },
                    ..column(theme, 0.0)
                },
                Visual::solid(colors.well, 0.0),
                &[child],
            )?);
        }
    }
    let radius = theme.px(theme.radius * 1.5);
    // 留出边框那一圈，展开区的浅底不盖住边框
    let border = LengthPercentage::length(theme.px(1.0));
    let node = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            padding: Rect {
                left: border,
                right: border,
                top: border,
                bottom: border,
            },
            ..Style::default()
        },
        solid(
            Some(colors.surface),
            Some((theme.px(1.0), colors.line)),
            radius,
        ),
        &children,
    )?;
    ui.scene().set_clip(node, radius);
    Ok(node)
}
