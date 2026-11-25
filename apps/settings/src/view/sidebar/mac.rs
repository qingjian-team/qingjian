//! macOS 系统设置的侧栏：浮起的圆角面板（红绿灯在它上面），搜索框，各页一行（22 点的彩色圆角方块加白色线条图标），
//! 分三段再加最后的「关于」，最下面是界面语言。系统毛玻璃要平台壳配合，这里先画成纯色。

use std::sync::Arc;

use bamboo_slip::taffy::{
    Dimension, FlexDirection, LengthPercentage, LengthPercentageAuto, NodeId, Rect, Size, Style,
};
use bamboo_slip::{BoxPaint, Color, Fill, RenderError, ScrollArea, TITLEBAR, Ui, Visual};

use super::{language, logo, navigation, results, search_field};
use crate::message::Message;
use crate::page_id::PageId;
use crate::view::Page;

/// 面板宽度（点）。
pub const WIDTH: f32 = 232.0;

/// 面板离窗口左、上、下边的距离（点）。
pub const MARGIN: f32 = 7.0;

/// 侧栏的几段，段间留空，跟系统设置一样。
const SECTIONS: [&[PageId]; 4] = [
    &[
        PageId::Input,
        PageId::Candidates,
        PageId::Language,
        PageId::Appearance,
        PageId::Shortcuts,
    ],
    &[
        PageId::Phrases,
        PageId::Dictionaries,
        PageId::Cloud,
        PageId::Stats,
        PageId::Vocabulary,
    ],
    &[PageId::Advanced, PageId::Experimental],
    &[PageId::About],
];

pub fn sidebar(ui: &mut Ui<Message>, page: &Page) -> Result<NodeId, RenderError> {
    let search = search_field(ui, page)?;
    let mut items = Vec::new();
    if page.settings.search.active() {
        items = results(ui, page)?;
    } else {
        for (index, pages) in SECTIONS.iter().enumerate() {
            if index > 0 {
                items.push(blank(ui, page, 14.0)?);
            }
            items.extend(navigation(ui, page, pages)?);
        }
    }
    let list = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            ..Style::default()
        },
        Visual::Group,
        &items,
    )?;
    // 搜索结果一条两三行，多了要滚；导航放得下，滚不动
    let list = ScrollArea::new(Style {
        flex_grow: 1.0,
        flex_basis: Dimension::length(0.0),
        min_size: Size {
            width: LengthPercentageAuto::auto(),
            height: LengthPercentageAuto::length(0.0),
        },
        ..Style::default()
    })
    .build(ui, &page.theme, "sidebar", list)?;
    let language = language(ui, page)?;
    let side = LengthPercentage::length(page.px(10.0));
    let margin = LengthPercentageAuto::length(page.px(MARGIN));
    ui.scene().node(
        Style {
            size: Size {
                width: Dimension::length(page.px(WIDTH)),
                height: Dimension::auto(),
            },
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            gap: page.gap(8.0),
            margin: Rect {
                left: margin,
                top: margin,
                bottom: margin,
                right: LengthPercentageAuto::length(0.0),
            },
            padding: Rect {
                left: side,
                right: side,
                // 红绿灯画在面板顶上那一条里
                top: LengthPercentage::length(page.px(TITLEBAR + 14.0 - MARGIN)),
                bottom: LengthPercentage::length(page.px(10.0)),
            },
            ..Style::default()
        },
        Visual::Box(BoxPaint {
            fill: Some(if page.palette.backdrop {
                Fill::Cutout(page.palette.sidebar)
            } else {
                Fill::Solid(page.palette.sidebar)
            }),
            border: Some((page.px(0.5).max(1.0), page.theme.colors.line)),
            radius: page.px(18.0),
        }),
        &[search, list, language],
    )
}

/// 22 点的彩色圆角方块，上浅下深，中间白色的线条图标；「关于」是白底方块上的青简 logo，像应用图标。
pub fn tile(ui: &mut Ui<Message>, page: &Page, id: PageId) -> Result<NodeId, RenderError> {
    if id == PageId::About {
        let mark = logo(ui, page, 17.0, 22.0)?;
        let side = page.px(22.0);
        return ui.scene().node(
            Style {
                size: Size {
                    width: Dimension::length(side),
                    height: Dimension::length(side),
                },
                flex_shrink: 0.0,
                ..Style::default()
            },
            Visual::Box(BoxPaint {
                fill: Some(Fill::Solid(Color::rgb(0xFF, 0xFF, 0xFF))),
                border: Some((page.px(0.5).max(1.0), page.theme.colors.line)),
                radius: page.px(6.0),
            }),
            &[mark],
        );
    }
    let base = tint(id);
    let (side, glyph) = (page.px(22.0), page.px(14.0));
    let inset = (side - glyph) / 2.0;
    let icon = ui.scene().node(
        Style {
            size: Size {
                width: Dimension::length(glyph),
                height: Dimension::length(glyph),
            },
            margin: Rect {
                left: LengthPercentageAuto::length(inset),
                top: LengthPercentageAuto::length(inset),
                right: LengthPercentageAuto::length(0.0),
                bottom: LengthPercentageAuto::length(0.0),
            },
            ..Style::default()
        },
        Visual::Icon {
            icon: Arc::new(id.glyph()),
            size: glyph,
            color: Color::rgb(0xFF, 0xFF, 0xFF),
        },
        &[],
    )?;
    ui.scene().node(
        Style {
            size: Size {
                width: Dimension::length(side),
                height: Dimension::length(side),
            },
            flex_shrink: 0.0,
            ..Style::default()
        },
        Visual::Box(BoxPaint {
            fill: Some(Fill::Linear {
                angle: 180.0,
                stops: vec![
                    (base.lerp(Color::rgb(0xFF, 0xFF, 0xFF), 0.18), 0.0),
                    (base, 1.0),
                ],
            }),
            border: None,
            radius: page.px(6.0),
        }),
        &[icon],
    )
}

/// 各页方块的颜色，照系统设置的配色挑：同类的页颜色相近。
fn tint(id: PageId) -> Color {
    match id {
        PageId::Input => Color::rgb(0x8E, 0x8E, 0x93),
        PageId::Candidates => Color::rgb(0x30, 0xA4, 0x6C),
        PageId::Language => Color::rgb(0x2F, 0x6F, 0xED),
        PageId::Appearance => Color::rgb(0xD6, 0x40, 0x9F),
        PageId::Shortcuts | PageId::Advanced => Color::rgb(0x6E, 0x77, 0x87),
        PageId::Phrases => Color::rgb(0xF7, 0x6B, 0x15),
        PageId::Dictionaries => Color::rgb(0xA0, 0x52, 0x2D),
        PageId::Cloud => Color::rgb(0x00, 0x91, 0xFF),
        PageId::Stats => Color::rgb(0xE5, 0x48, 0x4D),
        PageId::Vocabulary => Color::rgb(0x8E, 0x4E, 0xC6),
        PageId::Experimental => Color::rgb(0x12, 0xA5, 0x94),
        PageId::About => Color::rgb(0x33, 0x6F, 0x33),
    }
}

fn blank(ui: &mut Ui<Message>, page: &Page, height: f32) -> Result<NodeId, RenderError> {
    ui.scene().node(
        Style {
            size: Size {
                width: Dimension::auto(),
                height: Dimension::length(page.px(height)),
            },
            ..Style::default()
        },
        Visual::Group,
        &[],
    )
}
