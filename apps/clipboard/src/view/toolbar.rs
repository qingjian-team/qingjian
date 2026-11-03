//! 面板顶上：搜索框与一排筛选标签。

use std::sync::Arc;

use bamboo_slip::taffy::{
    AlignItems, Dimension, FlexDirection, JustifyContent, NodeId, Size, Style,
};
use bamboo_slip::{BoxPaint, Button, Easing, Fill, Interact, RenderError, SearchField, Ui, Visual};
use qingjian_clipboard::ClipFilter;

use super::icons::SearchGlyph;
use super::{Context, gap, pad, text_node};
use crate::app::Message;
use crate::i18n::tr;

/// 搜索框的宽（点，横排时）。
const SEARCH_WIDTH: f32 = 280.0;

fn filter_label(filter: ClipFilter) -> &'static str {
    match filter {
        ClipFilter::All => tr("filter.all"),
        ClipFilter::Text => tr("filter.text"),
        ClipFilter::Code => tr("filter.code"),
        ClipFilter::Image => tr("filter.image"),
        ClipFilter::Files => tr("filter.files"),
    }
}

fn filter_chip(
    ui: &mut Ui<Message>,
    context: &Context,
    filter: ClipFilter,
) -> Result<NodeId, RenderError> {
    let theme = &context.theme;
    let colors = &theme.colors;
    let key = ui.key(("filter", filter));
    let selected = context.app.filter == filter;
    let (fill, text, border) = if selected {
        (colors.accent, colors.on_accent, None)
    } else if ui.state(key).pressed {
        (
            colors.well,
            colors.secondary,
            Some((theme.px(1.0), colors.line)),
        )
    } else if ui.state(key).hovered {
        (
            colors.well_hover,
            colors.text,
            Some((theme.px(1.0), colors.line)),
        )
    } else {
        (
            colors.button,
            colors.text,
            Some((theme.px(1.0), colors.line)),
        )
    };
    let label = text_node(
        ui,
        filter_label(filter).to_owned(),
        theme.text(theme.font.size, 400, text),
        Style::default(),
    )?;
    let node = ui.scene().node(
        Style {
            align_items: AlignItems::CENTER,
            padding: pad(theme, 0.0, 12.0),
            size: Size {
                width: Dimension::auto(),
                height: Dimension::length(theme.px(theme.control_height)),
            },
            flex_shrink: 0.0,
            ..Style::default()
        },
        Visual::Box(BoxPaint {
            fill: Some(Fill::Solid(fill)),
            border,
            radius: theme.px(theme.radius),
        }),
        &[label],
    )?;
    ui.fade(node, key, theme.hover, Easing::Ease);
    // 不拿焦点：点了筛选之后接着打字还是进搜索框
    ui.interact(node, key, Interact::new().on_click(Message::Filter(filter)));
    Ok(node)
}

pub(super) fn toolbar(ui: &mut Ui<Message>, context: &Context) -> Result<NodeId, RenderError> {
    let theme = &context.theme;
    let horizontal = context.app.horizontal();
    let search = SearchField::new(&context.app.search)
        .placeholder(tr("panel.search"))
        .icon(Arc::new(SearchGlyph))
        .autofocus()
        .build(
            ui,
            theme,
            "search",
            Message::Search,
            None,
            Some(Message::Close),
        )?;
    // 搜索框撑满父节点，所以宽度定在外面这层
    let search_box = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            size: Size {
                width: if horizontal {
                    Dimension::length(theme.px(SEARCH_WIDTH))
                } else {
                    Dimension::percent(1.0)
                },
                height: Dimension::auto(),
            },
            ..Style::default()
        },
        Visual::Group,
        &[search],
    )?;
    let mut chips = Vec::with_capacity(ClipFilter::ALL.len());
    for filter in ClipFilter::ALL {
        chips.push(filter_chip(ui, context, filter)?);
    }
    let filters = ui.scene().node(
        Style {
            align_items: AlignItems::CENTER,
            gap: gap(theme, 6.0),
            flex_shrink: 0.0,
            ..Style::default()
        },
        Visual::Group,
        &chips,
    )?;
    let mut children = vec![search_box, filters];
    if context.app.link.service().is_some() {
        children.push(Button::new(tr("devices.button")).build(
            ui,
            theme,
            "open-devices",
            Message::OpenDevices,
        )?);
    }
    if context.app.paused {
        children.push(text_node(
            ui,
            tr("panel.paused").to_owned(),
            theme.text(theme.small, 400, theme.colors.secondary),
            Style::default(),
        )?);
    }
    ui.scene().node(
        Style {
            flex_shrink: 0.0,
            flex_direction: if horizontal {
                FlexDirection::Row
            } else {
                FlexDirection::Column
            },
            justify_content: JustifyContent::CENTER,
            align_items: AlignItems::CENTER,
            gap: gap(theme, 10.0),
            ..Style::default()
        },
        Visual::Group,
        &children,
    )
}
