//! 视图：竹青侧栏（logo、搜索、线性图标的导航、竹节标记）+ 右边当前页的设置项，整页可滚。
//! 哪页放哪些项见 docs/design/settings.md。

mod content;
mod help;
pub mod icons;
pub mod look;
mod page;
mod pages;
pub mod palette;
mod parts;
pub mod previews;
mod sidebar;

pub use page::Page;

use bamboo_slip::FontSpec;
use bamboo_slip::taffy::{Dimension, FlexDirection, NodeId, Size, Style};
use bamboo_slip::{
    Interact, Interaction, KeyCode, Look, RenderError, Ui, Visual, WidgetColors, WidgetTheme,
};

use crate::i18n::tr;
use crate::message::Message;
use crate::page_id::PageId;
use crate::search::Entry;
use crate::settings::Settings;
use palette::palette;

/// 照系统设置的两套用系统的字号；竹青那套比引擎缺省大一档：13 点的正文在设置里读着吃力。
fn widget_theme(look: Look, colors: WidgetColors, scale: f32) -> WidgetTheme {
    match look {
        Look::Mac => WidgetTheme::mac(colors, scale),
        Look::Fluent => WidgetTheme::fluent(colors, scale),
        Look::Plain => WidgetTheme {
            font: FontSpec::new(15.0, 21.0),
            small: 12.5,
            control_height: 28.0,
            ..WidgetTheme::new(colors, scale)
        },
    }
}

pub fn view(
    ui: &mut Ui<Message>,
    settings: &Settings,
    scale: f32,
    (width, height): (u32, u32),
) -> Result<NodeId, RenderError> {
    let look = look::current();
    let palette = palette(look, settings.system_dark, settings.backdrop);
    let page = Page {
        settings,
        theme: widget_theme(look, palette.widgets, scale),
        palette: &palette,
        scale,
        content_width: content_width(look, width as f32 / scale) * scale,
        window: (width as f32, height as f32),
        expand_all: false,
    };
    let current = settings.page;
    match look {
        Look::Plain => ui.info_links(tr("common.info_hint"), move |title| {
            crate::docs::link(current, title).map(Message::OpenDoc)
        }),
        Look::Mac | Look::Fluent => {
            let hint = if look == Look::Mac {
                tr("common.learn_more")
            } else {
                tr("common.learn_more_details")
            };
            ui.info_links(hint, move |title| {
                crate::docs::inline_link(current, title).map(Message::OpenDoc)
            });
        }
    }
    ui.row_icons(move |title| icons::row_icon(current, title));
    let sidebar = sidebar::sidebar(ui, &page)?;
    let content = content::content(ui, &page)?;
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
    let look = look::current();
    let palette = palette(look, false, false);
    let page = Page {
        settings,
        theme: widget_theme(look, palette.widgets, 1.0),
        palette: &palette,
        scale: 1.0,
        content_width: content_width(look, look::window_size(look).0),
        window: look::window_size(look),
        expand_all: true,
    };
    let mut entries = Vec::new();
    for id in PageId::ALL.into_iter().filter(|id| id.shown()) {
        entries.push(Entry {
            page: id,
            key: None,
            title: id.label().to_owned(),
            detail: None,
            keywords: Vec::new(),
            region: None,
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
            region: item.region,
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
        PageId::Experimental => pages::experimental::experimental(ui, page)?,
        PageId::About => pages::about::about(ui, page)?,
    })
}

/// 右边内容一栏扣掉左右边距后的宽度（点），`window` 是窗口宽（点）。
fn content_width(look: Look, window: f32) -> f32 {
    let width = window - sidebar::width(look) - content::padding(look) * 2.0;
    content::max_width(look).map_or(width, |max| width.min(max))
}
