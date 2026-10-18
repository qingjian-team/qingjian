//! 视图：竹青侧栏（logo、搜索、线性图标的导航、竹节标记）+ 右边当前页的设置项，整页可滚。
//! 哪页放哪些项见 docs/design/settings.md。

pub mod icons;
mod page;
mod pages;
pub mod palette;
mod parts;
pub mod previews;
mod sidebar;

pub use page::Page;

use bamboo_slip::FontSpec;
use bamboo_slip::taffy::{Dimension, FlexDirection, LengthPercentage, NodeId, Rect, Size, Style};
use bamboo_slip::{
    Interact, Interaction, KeyCode, RenderError, ScrollArea, TITLEBAR, Ui, Visual, WidgetColors,
    WidgetTheme,
};

use crate::message::Message;
use crate::page_id::PageId;
use crate::search::Entry;
use crate::settings::Settings;
use palette::{DARK, LIGHT};

/// 右边内容区左右的边距（点）。
const CONTENT_PADDING: f32 = 36.0;

/// 设置程序的字号比引擎缺省大一档：13 点的正文在设置里读着吃力。
fn widget_theme(colors: WidgetColors, scale: f32) -> WidgetTheme {
    WidgetTheme {
        font: FontSpec::new(15.0, 21.0),
        small: 12.5,
        control_height: 28.0,
        ..WidgetTheme::new(colors, scale)
    }
}

pub fn view(
    ui: &mut Ui<Message>,
    settings: &Settings,
    scale: f32,
    (width, height): (u32, u32),
) -> Result<NodeId, RenderError> {
    let palette = if settings.system_dark { &DARK } else { &LIGHT };
    let page = Page {
        settings,
        theme: widget_theme(palette.widgets, scale),
        palette,
        scale,
        content_width: width as f32 - (sidebar::WIDTH + CONTENT_PADDING * 2.0) * scale,
    };
    let current = settings.page;
    ui.info_links("点一下 ⓘ 打开文档", move |title| {
        crate::docs::link(current, title).map(Message::OpenDoc)
    });
    let sidebar = sidebar::sidebar(ui, &page)?;
    let content = content(ui, &page)?;
    let mut children = vec![sidebar, content];
    if let Some(popup) = settings.menu
        && let Some(menu) = parts::menus::menu(ui, &page, popup, (width as f32, height as f32))?
    {
        children.push(menu);
    }
    let root = ui.scene().node(
        Style {
            size: Size {
                width: Dimension::length(width as f32),
                height: Dimension::length(height as f32),
            },
            flex_direction: FlexDirection::Row,
            ..Style::default()
        },
        Visual::solid(palette.window, 0.0),
        &children,
    )?;
    let key = ui.key("root");
    let menu_open = settings.menu.is_some();
    let dialog_open = settings.phrase.is_some();
    ui.interact(
        root,
        key,
        Interact::new().on_key(move |press| {
            if menu_open && press.key == KeyCode::Escape {
                return Some(Message::CloseMenu);
            }
            if dialog_open && press.key == KeyCode::Escape {
                return Some(Message::ClosePhrase);
            }
            let close = press.key == KeyCode::Escape
                || (press.modifiers.command && press.key == KeyCode::Character("w".to_owned()));
            close.then_some(Message::Quit)
        }),
    );
    Ok(root)
}

/// 搜索索引：离屏把每页建一遍，收各页登记的可搜索项（key 与真建时一致），再加上页名本身。
pub fn index(settings: &Settings) -> Vec<Entry> {
    let interaction = Interaction::new();
    let page = Page {
        settings,
        theme: widget_theme(LIGHT.widgets, 1.0),
        palette: &LIGHT,
        scale: 1.0,
        content_width: 600.0,
    };
    let mut entries = Vec::new();
    for id in PageId::ALL {
        entries.push(Entry {
            page: id,
            key: None,
            title: id.label().to_owned(),
            detail: None,
            keywords: Vec::new(),
        });
        let mut ui = Ui::new(&interaction);
        if let Err(error) = page_children(&mut ui, &page, id) {
            tracing::warn!(%error, page = id.label(), "建搜索索引时这页建失败");
        }
        entries.extend(ui.take_searchables().into_iter().map(|item| Entry {
            page: id,
            key: Some(item.key),
            title: item.title,
            detail: item.detail,
            keywords: item.keywords,
        }));
    }
    entries
}

fn page_children(
    ui: &mut Ui<Message>,
    page: &Page,
    id: PageId,
) -> Result<Vec<NodeId>, RenderError> {
    Ok(match id {
        PageId::Input => pages::input::input(ui, page)?,
        PageId::Candidates => pages::candidates::candidates(ui, page)?,
        PageId::Language => pages::language::language(ui, page)?,
        PageId::Appearance => pages::appearance::appearance(ui, page)?,
        PageId::Shortcuts => pages::shortcuts::shortcuts(ui, page)?,
        PageId::Phrases => pages::phrases::phrases(ui, page)?,
        PageId::Dictionaries => pages::dictionaries::dictionaries(ui, page)?,
        PageId::Cloud => pages::cloud::cloud(ui, page)?,
        PageId::Stats => pages::stats::stats(ui, page)?,
        PageId::Vocabulary => pages::vocabulary::vocabulary(ui, page)?,
        PageId::Advanced => pages::advanced::advanced(ui, page)?,
        PageId::About => pages::about::about(ui, page)?,
    })
}

fn content(ui: &mut Ui<Message>, page: &Page) -> Result<NodeId, RenderError> {
    let children = page_children(ui, page, page.settings.page)?;
    let column = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            gap: page.gap(10.0),
            padding: Rect {
                left: LengthPercentage::length(page.px(CONTENT_PADDING)),
                right: LengthPercentage::length(page.px(CONTENT_PADDING)),
                top: LengthPercentage::length(page.px(TITLEBAR + 12.0)),
                bottom: LengthPercentage::length(page.px(28.0)),
            },
            ..Style::default()
        },
        Visual::Group,
        &children,
    )?;
    // 每页各记各的滚动位置
    ScrollArea::new(Style {
        flex_grow: 1.0,
        ..Style::default()
    })
    .build(ui, &page.theme, ("content", page.settings.page), column)
}
