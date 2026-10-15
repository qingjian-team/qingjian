//! 侧栏：logo 与「青简」、搜索框、各页的导航项；当前页左边叠一段竹节。

use std::sync::Arc;

use bamboo_slip::taffy::{
    Dimension, FlexDirection, JustifyContent, LengthPercentage, LengthPercentageAuto, NodeId,
    Position, Rect, Size, Style,
};
use bamboo_slip::{
    BoxPaint, Fill, FontSpec, FontWeight, NavItem, RenderError, SearchField, TITLEBAR, TextStyle,
    Ui, Visual, paragraph,
};

use super::Page;
use crate::message::Message;
use crate::page_id::PageId;
use crate::view::icons::Glyph;

/// 侧栏宽度（点）。
pub const WIDTH: f32 = 216.0;

pub fn sidebar(ui: &mut Ui<Message>, page: &Page) -> Result<NodeId, RenderError> {
    let theme = &page.theme;
    let brand = brand(ui, page)?;
    let search = SearchField::new(&page.settings.search.edit)
        .placeholder("搜索")
        .icon(Arc::new(Glyph::Search))
        .build(
            ui,
            theme,
            "search",
            Message::Search,
            Some(Message::SearchFirst),
            Some(Message::ClearSearch),
        )?;
    let items = if page.settings.search.active() {
        results(ui, page)?
    } else {
        navigation(ui, page)?
    };
    let list = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            gap: page.gap(1.0),
            margin: Rect {
                top: LengthPercentageAuto::length(page.px(12.0)),
                ..Rect::zero()
            },
            ..Style::default()
        },
        Visual::Group,
        &items,
    )?;
    let (vertical, horizontal) = (page.px(16.0), page.px(14.0));
    ui.scene().node(
        Style {
            size: Size {
                width: Dimension::length(page.px(WIDTH)),
                height: Dimension::auto(),
            },
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            gap: page.gap(14.0),
            padding: Rect {
                left: LengthPercentage::length(horizontal),
                right: LengthPercentage::length(horizontal),
                top: LengthPercentage::length(page.px(TITLEBAR + 8.0)),
                bottom: LengthPercentage::length(vertical),
            },
            ..Style::default()
        },
        Visual::solid(page.palette.sidebar, 0.0),
        &[brand, search, list],
    )
}

/// 各页的导航项，当前页左边叠竹节。
fn navigation(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    ui.scope("nav", |ui| -> Result<Vec<NodeId>, RenderError> {
        let mut items = Vec::new();
        for id in PageId::ALL {
            let selected = id == page.settings.page;
            let mut item = NavItem::new(id.label())
                .icon(Arc::new(id.glyph()))
                .selected(selected);
            if selected {
                item = item.accessory(joint(ui, page)?);
            }
            items.push(item.build(ui, theme, id, Message::Page(id))?);
        }
        Ok(items)
    })
}

/// 搜索结果：图标是所在页的，点了跳过去；一条都没有时说一声。
fn results(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let hits = &page.settings.search.hits;
    if hits.is_empty() {
        return Ok(vec![paragraph(
            ui,
            theme,
            "没有找到",
            Some(theme.colors.secondary),
        )?]);
    }
    ui.scope("hits", |ui| -> Result<Vec<NodeId>, RenderError> {
        let mut items = Vec::new();
        for (index, hit) in hits.iter().enumerate() {
            items.push(
                NavItem::new(&hit.title)
                    .icon(Arc::new(hit.page.glyph()))
                    .build(ui, theme, index, Message::Jump(index))?,
            );
        }
        Ok(items)
    })
}

/// logo 加宋体的「青简」。
fn brand(ui: &mut Ui<Message>, page: &Page) -> Result<NodeId, RenderError> {
    let mut children = Vec::new();
    if let Some(logo) = &page.settings.logo {
        let height = page.px(24.0);
        let (width, natural) = logo.size();
        children.push(ui.scene().node(
            Style {
                size: Size {
                    width: Dimension::length(height * width / natural),
                    height: Dimension::length(height),
                },
                ..Style::default()
            },
            Visual::Box(BoxPaint {
                fill: Some(Fill::Svg {
                    image: Arc::clone(logo),
                    slice: None,
                    px_per_unit: height / natural,
                }),
                border: None,
                radius: 0.0,
            }),
            &[],
        )?);
    }
    let font = FontSpec {
        weight: FontWeight(700),
        family: page.settings.serif,
        ..FontSpec::new(19.0, 26.0)
    };
    children.push(ui.scene().node(
        Style::default(),
        Visual::Text {
            text: "青简".to_owned(),
            style: TextStyle::new(font.scaled(page.scale), 19.0, page.theme.colors.text, 1.0),
        },
        &[],
    )?);
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            align_items: Some(bamboo_slip::taffy::AlignItems::CENTER),
            gap: page.gap(8.0),
            padding: Rect {
                left: LengthPercentage::length(page.px(6.0)),
                ..Rect::zero()
            },
            ..Style::default()
        },
        Visual::Group,
        &children,
    )
}

/// 当前页的竹节：选中项左边一段中间断开的竖线。
fn joint(ui: &mut Ui<Message>, page: &Page) -> Result<NodeId, RenderError> {
    let (thick, length) = (page.px(2.0), page.px(16.0));
    let part = length * 0.42;
    let mut pieces = Vec::new();
    for top in [0.0, length - part] {
        pieces.push(ui.scene().node(
            Style {
                position: Position::Absolute,
                size: Size {
                    width: Dimension::length(thick),
                    height: Dimension::length(part),
                },
                inset: Rect {
                    left: LengthPercentageAuto::length(0.0),
                    top: LengthPercentageAuto::length(top),
                    right: LengthPercentageAuto::auto(),
                    bottom: LengthPercentageAuto::auto(),
                },
                ..Style::default()
            },
            Visual::solid(page.theme.colors.accent, thick / 2.0),
            &[],
        )?);
    }
    let column = ui.scene().node(
        Style {
            size: Size {
                width: Dimension::length(thick),
                height: Dimension::length(length),
            },
            ..Style::default()
        },
        Visual::Group,
        &pieces,
    )?;
    ui.scene().node(
        Style {
            position: Position::Absolute,
            flex_direction: FlexDirection::Column,
            justify_content: Some(JustifyContent::CENTER),
            inset: Rect {
                left: LengthPercentageAuto::length(-page.px(9.0)),
                top: LengthPercentageAuto::length(0.0),
                right: LengthPercentageAuto::auto(),
                bottom: LengthPercentageAuto::length(0.0),
            },
            ..Style::default()
        },
        Visual::Group,
        &[column],
    )
}
