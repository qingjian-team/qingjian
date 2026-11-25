//! 侧栏：搜索框、各页的导航项、界面语言。三种外观各一个文件（竹青、macOS 系统设置、Windows 11 设置），
//! 搜索框、搜索结果、界面语言与页面图标共用这里的。

mod fluent;
mod mac;
mod plain;

use std::sync::Arc;

use bamboo_slip::taffy::{
    AlignItems, Dimension, FlexDirection, JustifyContent, NodeId, Size, Style,
};
use bamboo_slip::{
    BoxPaint, Fill, Interact, KeyCode, Look, NavItem, RenderError, SearchField, Suggestion,
    Suggestions, Ui, Visual, paragraph,
};

use super::Page;
use crate::i18n::tr;
use crate::message::{Message, Popup};
use crate::page_id::PageId;
use crate::view::icons::Glyph;
use crate::view::parts::menus::{anchor, ui_language, ui_language_options};

/// 搜索框的 key：Windows 的建议列表按它的位置摆。
const SEARCH: &str = "search";

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

/// 搜索框：回车跳到结果里选到的那条（没选过是第一条），Esc 清空；↑ ↓ 在结果里移（Windows 的建议列表才画出选到的那条）。
pub(super) fn search_field(ui: &mut Ui<Message>, page: &Page) -> Result<NodeId, RenderError> {
    let field = SearchField::new(&page.settings.search.edit)
        .placeholder(if page.theme.look == Look::Fluent {
            tr("sidebar.find_setting")
        } else {
            tr("sidebar.search")
        })
        .icon(Arc::new(Glyph::Search))
        .build(
            ui,
            &page.theme,
            SEARCH,
            Message::Search,
            Some(Message::SearchCommit),
            Some(Message::ClearSearch),
        )?;
    // 单行输入框不接 ↑ ↓，按键冒到这一层
    let node = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            ..Style::default()
        },
        Visual::Group,
        &[field],
    )?;
    let key = ui.key("search-keys");
    ui.interact(
        node,
        key,
        Interact::new().on_key(|press| match press.key {
            KeyCode::ArrowDown => Some(Message::SearchStep(true)),
            KeyCode::ArrowUp => Some(Message::SearchStep(false)),
            _ => None,
        }),
    );
    Ok(node)
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
    let mac = theme.look == Look::Mac;
    ui.scope("hits", |ui| -> Result<Vec<NodeId>, RenderError> {
        let mut items = Vec::new();
        for (index, hit) in hits.iter().enumerate() {
            let mut item = NavItem::new(&hit.title);
            // 系统设置的写法：说明里命中的摘一段，下面再写在哪一页
            if mac {
                if let Some(snippet) = &hit.snippet {
                    item = item.line(snippet);
                }
                if hit.key.is_some() {
                    item = item.line(hit.page.label());
                }
            }
            let item = decorate(ui, page, item, hit.page)?;
            items.push(item.build(ui, theme, index, Message::Jump(index))?);
        }
        Ok(items)
    })
}

/// Windows 的搜索结果：搜索框下面弹出的建议列表，一条是标题、命中的那段说明与所在页；挂在根上盖住导航。
/// 位置按上一帧量出的搜索框摆，刚开始搜的第一帧还量不到时下一帧再出。
pub(super) fn suggestions(ui: &mut Ui<Message>, page: &Page) -> Result<(), RenderError> {
    let search = &page.settings.search;
    if !search.active() {
        return Ok(());
    }
    let Some(anchor) = ui.rect(ui.key(SEARCH)) else {
        return Ok(());
    };
    let items: Vec<Suggestion> = search
        .hits
        .iter()
        .map(|hit| Suggestion {
            title: &hit.title,
            detail: hit.snippet.as_deref(),
            trailing: hit.key.is_some().then(|| hit.page.label()),
        })
        .collect();
    let list = Suggestions::new(&items, anchor, page.window)
        .hot(search.hot)
        .empty(tr("sidebar.no_results"))
        .build(ui, &page.theme, "suggestions", Message::Jump)?;
    ui.overlay(list);
    Ok(())
}

/// 青简的 logo，高 `height` 点、宽按原图比例，放在 `side` 点见方的格子正中；logo 没解析出来时格子空着。
pub(super) fn logo(
    ui: &mut Ui<Message>,
    page: &Page,
    height: f32,
    side: f32,
) -> Result<NodeId, RenderError> {
    let mut children = Vec::new();
    if let Some(logo) = &page.settings.logo {
        let height = page.px(height);
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
        &children,
    )
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
