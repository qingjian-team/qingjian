//! 候选窗预览卡与主题卡片：都由 qingjian-render 真画，跟着外观、排布、字号变。

use std::sync::Arc;

use bamboo_slip::taffy::{Dimension, FlexDirection, FlexWrap, JustifyContent, NodeId, Size, Style};
use bamboo_slip::{BoxPaint, Easing, Fill, Interact, RenderError, Ui, Visual};
use qingjian_platform::LayoutMode;
use qingjian_render::Pixmap;

use crate::message::{Message, SizeKind};
use crate::view::Page;

/// 主题卡片最窄多大（点）：一排放得下几张就放几张，再一起拉宽把这一排铺满，高与缩略图跟着等比放大。
const TILE: (f32, f32) = (144.0, 76.0);

/// 卡片之间的间距（点）。
const TILE_GAP: f32 = 12.0;

/// 卡片所在分组左右的内边距（点），见 `bamboo_slip::group`。
const GROUP_PADDING: f32 = 16.0;

const TILE_PADDING: f32 = 8.0;

/// 预览卡的高度（点），候选窗等比缩进去、上下留边。
const PREVIEW_HEIGHT: f32 = 200.0;

const PREVIEW_PADDING: f32 = 12.0;

/// 预览卡：当前主题、外观、排布、字号下敲 nihao 的候选窗。
pub fn preview(ui: &mut Ui<Message>, page: &Page) -> Result<NodeId, RenderError> {
    let settings = page.settings;
    let general = &settings.config.general;
    let theme = settings.theme(general.theme_id());
    let fit = (
        page.content_width - page.px(PREVIEW_PADDING * 2.0),
        page.px(PREVIEW_HEIGHT - PREVIEW_PADDING * 2.0),
    );
    let image = settings.previews.borrow_mut().fitted(
        &theme,
        settings.candidate_dark(),
        general.layout == LayoutMode::Horizontal,
        5,
        page.scale,
        [
            settings.size(SizeKind::Candidate),
            settings.size(SizeKind::Annotation),
        ],
        fit,
    );
    let mut children = Vec::new();
    if let Some(pixmap) = image {
        children.push(picture(ui, pixmap)?);
    }
    let colors = &page.theme.colors;
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            justify_content: Some(JustifyContent::CENTER),
            size: Size {
                width: Dimension::auto(),
                height: Dimension::length(page.px(PREVIEW_HEIGHT)),
            },
            flex_shrink: 0.0,
            align_items: Some(bamboo_slip::taffy::AlignItems::CENTER),
            ..Style::default()
        },
        Visual::Box(BoxPaint {
            fill: Some(Fill::Linear {
                angle: 120.0,
                stops: vec![(page.palette.hero[0], 0.0), (page.palette.hero[1], 0.8)],
            }),
            border: Some((page.px(1.0), colors.line)),
            radius: page.px(14.0),
        }),
        &children,
    )
}

/// 各主题一张卡片：横排缩略图加名字，选中的描强调色边；点了换主题。
pub fn cards(ui: &mut Ui<Message>, page: &Page) -> Result<NodeId, RenderError> {
    let settings = page.settings;
    let current = settings.config.general.theme_id().to_owned();
    let colors = page.theme.colors;
    let (width, height) = tile_size(page);
    let zoom = width / page.px(TILE.0);
    let mut cards = Vec::new();
    for listed in settings.themes.themes() {
        let id = listed.id().to_owned();
        let theme = settings.theme(&id);
        let key = ui.scope("themes", |ui| ui.key(&id));
        let state = ui.state(key);
        let selected = id == current;
        let fit = (
            width - page.px(TILE_PADDING * 2.0),
            height - page.px(TILE_PADDING * 2.0),
        );
        let image = settings.previews.borrow_mut().fitted(
            &theme,
            settings.candidate_dark(),
            true,
            3,
            page.scale * 0.5 * zoom,
            [0.0, 0.0],
            fit,
        );
        let mut tile_children = Vec::new();
        if let Some(pixmap) = image {
            tile_children.push(picture(ui, pixmap)?);
        }
        let border = if selected {
            (page.px(2.0), colors.accent)
        } else if state.hovered {
            (page.px(1.0), colors.secondary)
        } else {
            (page.px(1.0), colors.line)
        };
        let tile = ui.scene().node(
            Style {
                flex_direction: FlexDirection::Row,
                justify_content: Some(JustifyContent::CENTER),
                align_items: Some(bamboo_slip::taffy::AlignItems::CENTER),
                size: Size {
                    width: Dimension::length(width),
                    height: Dimension::length(height),
                },
                ..Style::default()
            },
            Visual::Box(BoxPaint {
                fill: Some(Fill::Solid(page.palette.tile)),
                border: Some(border),
                radius: page.px(12.0),
            }),
            &tile_children,
        )?;
        let (weight, color) = if selected {
            (600, colors.selected_text)
        } else {
            (400, colors.text)
        };
        let name = ui.scene().node(
            Style::default(),
            Visual::Text {
                text: listed.name().to_owned(),
                style: page.theme.text(page.theme.small, weight, color),
            },
            &[],
        )?;
        let card = ui.scene().node(
            Style {
                flex_direction: FlexDirection::Column,
                align_items: Some(bamboo_slip::taffy::AlignItems::CENTER),
                gap: page.gap(6.0),
                ..Style::default()
            },
            Visual::Group,
            &[tile, name],
        )?;
        ui.transition(tile, key, page.theme.hover, Easing::Ease);
        ui.interact(
            card,
            key,
            Interact::new()
                .on_click(Message::set("general", "theme", id))
                .focusable(),
        );
        cards.push(card);
    }
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            gap: page.gap(TILE_GAP),
            ..Style::default()
        },
        Visual::Group,
        &cards,
    )
}

/// 主题卡片的宽高（像素）：按分组里能用的宽度算一排放几张，再均分这一排。
fn tile_size(page: &Page) -> (f32, f32) {
    let available = page.content_width - page.px(GROUP_PADDING * 2.0);
    let (min, gap) = (page.px(TILE.0), page.px(TILE_GAP));
    let columns = ((available + gap) / (min + gap)).floor().max(1.0);
    let width = ((available - gap * (columns - 1.0)) / columns).max(min);
    (width, width * TILE.1 / TILE.0)
}

/// 一张位图按原像素贴（已按要放的地方画好大小）。
fn picture(ui: &mut Ui<Message>, pixmap: Arc<Pixmap>) -> Result<NodeId, RenderError> {
    ui.scene().node(
        Style {
            size: Size {
                width: Dimension::length(pixmap.width() as f32),
                height: Dimension::length(pixmap.height() as f32),
            },
            flex_shrink: 0.0,
            ..Style::default()
        },
        Visual::Box(BoxPaint {
            fill: Some(Fill::Image {
                pixmap,
                slice: None,
                pixels_per_px: 1.0,
            }),
            border: None,
            radius: 0.0,
        }),
        &[],
    )
}
