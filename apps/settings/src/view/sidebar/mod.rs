//! 侧栏：搜索框、各页的导航项、界面语言。三种外观各一个文件（竹青、macOS 系统设置、Windows 11 设置），
//! 搜索框、搜索结果、界面语言与页面图标共用这里的。

mod fluent;
mod mac;
mod plain;

use std::sync::Arc;

use bamboo_slip::taffy::{AlignItems, Dimension, JustifyContent, NodeId, Size, Style};
use bamboo_slip::{Look, NavItem, RenderError, SearchField, Ui, Visual, paragraph};

use super::Page;
use crate::i18n::tr;
use crate::message::{Message, Popup};
use crate::page_id::PageId;
use crate::view::icons::Glyph;
use crate::view::parts::menus::{anchor, ui_language, ui_language_options};

pub fn sidebar(ui: &mut Ui<Message>, page: &Page) -> Result<NodeId, RenderError> {
    match page.theme.look {
        Look::Plain => plain::sidebar(ui, page),
        Look::Mac => mac::sidebar(ui, page),
        Look::Fluent => fluent::sidebar(ui, page),
    }
}

/// 侧栏占的宽度（点），含外边距：右边内容区按它算宽度。
pub fn width(look: Look) -> f32 {
    match look {
        Look::Plain => plain::WIDTH,
        Look::Mac => mac::WIDTH + mac::MARGIN,
        Look::Fluent => fluent::WIDTH,
    }
}

pub(super) fn search_field(ui: &mut Ui<Message>, page: &Page) -> Result<NodeId, RenderError> {
    SearchField::new(&page.settings.search.edit)
        .placeholder(tr("sidebar.search"))
        .icon(Arc::new(Glyph::Search))
        .build(
            ui,
            &page.theme,
            "search",
            Message::Search,
            Some(Message::SearchFirst),
            Some(Message::ClearSearch),
        )
}

/// 一页的导航项配上这页的图标：竹青是线条图标，macOS 是彩色圆角方块，Windows 是 Fluent 彩色图标。
pub(super) fn decorate<'a>(
    ui: &mut Ui<Message>,
    page: &Page,
    item: NavItem<'a>,
    id: PageId,
) -> Result<NavItem<'a>, RenderError> {
    Ok(match page.theme.look {
        Look::Plain => item.icon(Arc::new(id.glyph())),
        Look::Mac => item.leading(mac::tile(ui, page, id)?),
        Look::Fluent => item.leading(fluent::icon(ui, page, id)?),
    })
}

/// `pages` 里这个平台有的页，一页一个导航项。
pub(super) fn navigation(
    ui: &mut Ui<Message>,
    page: &Page,
    pages: &[PageId],
) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    ui.scope("nav", |ui| -> Result<Vec<NodeId>, RenderError> {
        let mut items = Vec::new();
        for &id in pages.iter().filter(|id| id.shown()) {
            let selected = id == page.settings.page;
            let item = decorate(ui, page, NavItem::new(id.label()).selected(selected), id)?;
            items.push(item.build(ui, theme, id, Message::Page(id))?);
        }
        Ok(items)
    })
}

/// 界面语言：「文A」图标加当前语言自己的叫法，看不懂当前界面的人也认得出；点开是语言菜单。
pub(super) fn language(ui: &mut Ui<Message>, page: &Page) -> Result<NodeId, RenderError> {
    let general = &page.settings.config.general;
    let current = ui_language(general);
    let options = ui_language_options();
    let label = options
        .iter()
        .find(|(_, tag)| *tag == current)
        .map_or(current, |(label, _)| label);
    let item = match page.theme.look {
        Look::Plain => NavItem::new(label).icon(Arc::new(Glyph::Translate)),
        // 图标占的宽与上面各页的图标一样，字才对得齐
        Look::Mac => NavItem::new(label).leading(slot(ui, page, 22.0)?),
        Look::Fluent => NavItem::new(label).leading(slot(ui, page, 20.0)?),
    };
    item.build(
        ui,
        &page.theme,
        anchor(Popup::UiLanguage),
        Message::OpenMenu(Popup::UiLanguage),
    )
}

/// 搜索结果：图标是所在页的，点了跳过去；一条都没有时说一声。
pub(super) fn results(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let hits = &page.settings.search.hits;
    if hits.is_empty() {
        return Ok(vec![paragraph(
            ui,
            theme,
            tr("sidebar.no_results"),
            Some(theme.colors.secondary),
        )?]);
    }
    ui.scope("hits", |ui| -> Result<Vec<NodeId>, RenderError> {
        let mut items = Vec::new();
        for (index, hit) in hits.iter().enumerate() {
            let item = decorate(ui, page, NavItem::new(&hit.title), hit.page)?;
            items.push(item.build(ui, theme, index, Message::Jump(index))?);
        }
        Ok(items)
    })
}

/// 「文A」图标放在 `side` 点见方的格子正中。
fn slot(ui: &mut Ui<Message>, page: &Page, side: f32) -> Result<NodeId, RenderError> {
    let glyph = page.px(16.0);
    let icon = ui.scene().node(
        Style {
            size: Size {
                width: Dimension::length(glyph),
                height: Dimension::length(glyph),
            },
            ..Style::default()
        },
        Visual::Icon {
            icon: Arc::new(Glyph::Translate),
            size: glyph,
            color: page.theme.colors.text,
        },
        &[],
    )?;
    let side = page.px(side);
    ui.scene().node(
        Style {
            size: Size {
                width: Dimension::length(side),
                height: Dimension::length(side),
            },
            flex_shrink: 0.0,
            justify_content: JustifyContent::CENTER,
            align_items: AlignItems::CENTER,
            ..Style::default()
        },
        Visual::Group,
        &[icon],
    )
}
