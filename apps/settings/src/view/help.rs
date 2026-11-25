//! 每页最下面的帮助入口，打开官网上这页的文档：macOS 是右下角一个圆形的「?」，Windows 是「相关支持」一节里的「获取帮助」。
//! 竹青外观没有（文档从各行的 ⓘ 进）。

use std::sync::Arc;

use bamboo_slip::taffy::{
    AlignItems, Dimension, FlexDirection, JustifyContent, LengthPercentage, LengthPercentageAuto,
    NodeId, Rect, Size, Style,
};
use bamboo_slip::{
    BoxPaint, Cursor, Fill, Interact, Link, Look, RenderError, Ui, Visual, group, section_title,
};

use super::Page;
use super::icons::Glyph;
use crate::docs::page_link;
use crate::i18n::{tr, tr_args};
use crate::message::Message;

pub fn help(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let Some(url) = page_link(page.settings.page) else {
        return Ok(Vec::new());
    };
    match page.theme.look {
        Look::Plain => Ok(Vec::new()),
        Look::Mac => Ok(vec![question_mark(ui, page, url)?]),
        Look::Fluent => related_support(ui, page, url),
    }
}

/// 22 点的圆形按钮，中间一个「?」，靠右。
fn question_mark(ui: &mut Ui<Message>, page: &Page, url: String) -> Result<NodeId, RenderError> {
    let theme = &page.theme;
    let colors = &theme.colors;
    let key = ui.key("help");
    let state = ui.state(key);
    let ink = if state.hovered {
        colors.text
    } else {
        colors.secondary
    };
    let mark = ui.scene().node(
        Style::default(),
        Visual::Text {
            text: "?".to_owned(),
            style: theme.text(13.0, 600, ink),
        },
        &[],
    )?;
    let side = page.px(22.0);
    let button = ui.scene().node(
        Style {
            size: Size {
                width: Dimension::length(side),
                height: Dimension::length(side),
            },
            justify_content: JustifyContent::CENTER,
            align_items: AlignItems::CENTER,
            flex_shrink: 0.0,
            ..Style::default()
        },
        Visual::Box(BoxPaint {
            fill: Some(Fill::Solid(colors.button)),
            border: Some((page.px(0.5).max(1.0), colors.line)),
            radius: side / 2.0,
        }),
        &[mark],
    )?;
    ui.interact(
        button,
        key,
        Interact::new()
            .on_click(Message::OpenDoc(url))
            .focusable()
            .cursor(Cursor::Pointer),
    );
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FLEX_END,
            ..Style::default()
        },
        Visual::Group,
        &[button],
    )
}

/// 「相关支持」一节：一张卡片，左边信息图标加「获取帮助」，下面一行链接「在官网查看『某页』的说明」。
fn related_support(
    ui: &mut Ui<Message>,
    page: &Page,
    url: String,
) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let title = section_title(ui, theme, tr("common.related_support"))?;
    let side = page.px(20.0);
    let icon = ui.scene().node(
        Style {
            size: Size {
                width: Dimension::length(side),
                height: Dimension::length(side),
            },
            flex_shrink: 0.0,
            ..Style::default()
        },
        Visual::Icon {
            icon: Arc::new(Glyph::About),
            size: side,
            color: theme.colors.text,
        },
        &[],
    )?;
    let label = ui.scene().node(
        Style::default(),
        Visual::Text {
            text: tr("common.get_help").to_owned(),
            style: theme.text(theme.font.size, 400, theme.colors.text),
        },
        &[],
    )?;
    let heading = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            gap: page.gap(16.0),
            size: Size {
                width: Dimension::auto(),
                height: Dimension::length(page.px(36.0)),
            },
            ..Style::default()
        },
        Visual::Group,
        &[icon, label],
    )?;
    let text = tr_args("common.page_docs", &[("page", &page.settings.page.label())]);
    let link = Link::new(&text).build(ui, theme, "help", Message::OpenDoc(url))?;
    let link = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            margin: Rect {
                left: LengthPercentageAuto::length(side + page.px(16.0)),
                ..Rect::zero()
            },
            ..Style::default()
        },
        Visual::Group,
        &[link],
    )?;
    let body = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            padding: Rect {
                top: LengthPercentage::length(page.px(8.0)),
                bottom: LengthPercentage::length(page.px(12.0)),
                ..Rect::zero()
            },
            ..Style::default()
        },
        Visual::Group,
        &[heading, link],
    )?;
    Ok(vec![title, group(ui, theme, &[body])?])
}
