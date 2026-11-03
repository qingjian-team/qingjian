//! 面板的界面：上面搜索框与筛选，下面一排卡片。上下弹出时卡片横着排、横着滚，左右弹出时竖着排、竖着滚。

mod body;
mod card;
mod devices;
mod editor;
mod icons;
mod toolbar;

use bamboo_slip::taffy::{Dimension, FlexDirection, LengthPercentage, NodeId, Rect, Size, Style};
use bamboo_slip::{
    Interact, KeyCode, Menu, RenderError, ScrollArea, TextStyle, Ui, Visual, WidgetTheme,
};
use qingjian_clipboard::ClipItem;

use crate::app::{ClipboardApp, MENU_EDIT, Message, now_millis};
use crate::i18n::tr;
use crate::palette::Palette;

pub(crate) use card::row_key;

const PANEL_PADDING: f32 = 10.0;

const PANEL_GAP: f32 = 10.0;

/// 横排时卡片下面留给滚动条的高度（点）。
const BAR_ROOM: f32 = 10.0;

/// 建树时到处要用的东西。
pub(super) struct Context<'a> {
    pub app: &'a ClipboardApp,

    pub theme: WidgetTheme,

    pub palette: Palette,

    pub now: i64,

    /// 窗口大小（像素）。
    pub size: (f32, f32),
}

pub(super) fn pad(theme: &WidgetTheme, vertical: f32, horizontal: f32) -> Rect<LengthPercentage> {
    Rect {
        left: LengthPercentage::length(theme.px(horizontal)),
        right: LengthPercentage::length(theme.px(horizontal)),
        top: LengthPercentage::length(theme.px(vertical)),
        bottom: LengthPercentage::length(theme.px(vertical)),
    }
}

pub(super) fn gap(theme: &WidgetTheme, points: f32) -> Size<LengthPercentage> {
    Size {
        width: LengthPercentage::length(theme.px(points)),
        height: LengthPercentage::length(theme.px(points)),
    }
}

pub(super) fn text_node(
    ui: &mut Ui<Message>,
    text: String,
    style: TextStyle,
    layout: Style,
) -> Result<NodeId, RenderError> {
    ui.scene().node(layout, Visual::Text { text, style }, &[])
}

fn menu_items() -> [&'static str; 4] {
    [
        tr("menu.paste"),
        tr("menu.copy"),
        tr("menu.edit"),
        tr("menu.delete"),
    ]
}

fn list(ui: &mut Ui<Message>, context: &Context, size: (u32, u32)) -> Result<NodeId, RenderError> {
    let theme = &context.theme;
    let app = context.app;
    let horizontal = app.horizontal();
    // 横排时卡片高度算死：百分比在高度由内容决定的滚动内容里不生效，长文会把卡片撑出面板
    let extent = if horizontal {
        card::Extent::Tall(
            size.1 as f32
                - theme.px(PANEL_PADDING * 2.0 + PANEL_GAP + theme.control_height + BAR_ROOM),
        )
    } else {
        card::Extent::Wide(size.0 as f32 - theme.px(PANEL_PADDING * 2.0))
    };
    let mut cards = Vec::with_capacity(app.items.len());
    for (index, item) in app.items.iter().enumerate() {
        cards.push(card::card(ui, context, item, index, extent)?);
    }
    if cards.is_empty() {
        let hint = if app.search.text().is_empty() {
            tr("panel.empty")
        } else {
            tr("panel.no_results")
        };
        cards.push(text_node(
            ui,
            hint.to_owned(),
            theme.text(theme.font.size, 400, theme.colors.secondary),
            Style {
                padding: pad(theme, 6.0, 10.0),
                ..Style::default()
            },
        )?);
    }
    let content = ui.scene().node(
        Style {
            flex_direction: if horizontal {
                FlexDirection::Row
            } else {
                FlexDirection::Column
            },
            flex_grow: 1.0,
            gap: gap(theme, 10.0),
            ..Style::default()
        },
        Visual::Group,
        &cards,
    )?;
    let area = ScrollArea::new(Style {
        flex_grow: 1.0,
        flex_basis: Dimension::length(0.0),
        ..Style::default()
    });
    let area = if horizontal { area.horizontal() } else { area };
    area.build(ui, theme, ("list", app.list_generation), content)
}

pub fn view(
    ui: &mut Ui<Message>,
    app: &ClipboardApp,
    scale: f32,
    size: (u32, u32),
) -> Result<NodeId, RenderError> {
    let palette = Palette::new(app.dark, app.accent, app.acrylic);
    let window = palette.window;
    let mut theme = WidgetTheme::fluent(palette.widgets, scale);
    theme.font.family = app.ui_font;
    let context = Context {
        app,
        theme,
        palette,
        now: now_millis(),
        size: (size.0 as f32, size.1 as f32),
    };
    let theme = &context.theme;
    let mut children = match &app.editing {
        Some(editing) => editor::editor(ui, &context, editing, size)?,
        None if app.showing_devices => devices::devices(ui, &context)?,
        None => vec![toolbar::toolbar(ui, &context)?, list(ui, &context, size)?],
    };
    if let Some(menu) = &app.menu {
        let items = menu_items();
        let editable = app
            .items
            .get(menu.index)
            .is_some_and(|item| item.kind.editable());
        let disabled = [MENU_EDIT];
        let icons = icons::menu_icons();
        let (x, y) = menu.position;
        let node = Menu::new(
            &items,
            None,
            (x, y, 0.0, 0.0),
            (size.0 as f32, size.1 as f32),
        )
        .disabled(if editable { &[] } else { &disabled })
        .icons(&icons)
        .build(ui, theme, "menu", Message::MenuPick, Message::CloseMenu)?;
        children.push(node);
    }
    let root = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            size: Size {
                width: Dimension::length(size.0 as f32),
                height: Dimension::length(size.1 as f32),
            },
            padding: pad(theme, PANEL_PADDING, PANEL_PADDING),
            gap: gap(theme, PANEL_GAP),
            ..Style::default()
        },
        Visual::solid(window, 0.0),
        &children,
    )?;
    let editing = app.editing.is_some();
    let key = ui.key("root");
    ui.interact(
        root,
        key,
        Interact::new().on_key(move |press| match press.key {
            KeyCode::Escape if editing => Some(Message::EditCancel),
            _ if editing => None,
            KeyCode::ArrowUp => Some(Message::Move(false)),
            KeyCode::ArrowDown => Some(Message::Move(true)),
            KeyCode::Enter => Some(Message::PasteSelected),
            KeyCode::Escape => Some(Message::Close),
            _ => None,
        }),
    );
    Ok(root)
}

/// 程序路径里的程序名。
pub(super) fn app_name(item: &ClipItem) -> String {
    std::path::Path::new(&item.source)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}
