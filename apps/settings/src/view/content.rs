//! 右边的内容区，按外观排：竹青整页可滚；macOS 顶上浮着一条工具栏（前后箭头加页名），下面居中一栏从它底下滚过；
//! Windows 内容区顶上是 28 点的页标题，跟着整页一起滚。

use std::sync::Arc;

use bamboo_slip::taffy::{
    AlignItems, Dimension, FlexDirection, LengthPercentage, LengthPercentageAuto, NodeId, Position,
    Rect, Size, Style,
};
use bamboo_slip::{
    BoxPaint, Fill, Interact, Look, RenderError, ScrollArea, TITLEBAR, Ui, Visual, paragraph,
};

use super::Page;
use super::icons::Glyph;
use crate::i18n::tr_args;
use crate::message::Message;

/// macOS 工具栏的高度与下沿渐隐的高度（点）。
const TOOLBAR: f32 = 52.0;

const TOOLBAR_FADE: f32 = 14.0;

/// 内容区左右的边距（点）。
pub fn padding(look: Look) -> f32 {
    match look {
        Look::Plain => 36.0,
        Look::Mac => 24.0,
        Look::Fluent => 24.0,
    }
}

/// 内容一栏最宽多少点：窗口拉宽时 macOS 居中、Windows 靠左。
pub fn max_width(look: Look) -> Option<f32> {
    match look {
        Look::Plain => None,
        Look::Mac => Some(620.0),
        Look::Fluent => Some(1000.0),
    }
}

pub fn content(ui: &mut Ui<Message>, page: &Page) -> Result<NodeId, RenderError> {
    let look = page.theme.look;
    let mut children = Vec::new();
    if look == Look::Fluent {
        children.push(text(ui, page, page.settings.page.label(), 28.0, 600)?);
    }
    // 配置文件读不了时每页顶上都说一声：界面这时是缺省值，不是文件里写的
    if let Some(error) = &page.settings.config_error {
        let text = tr_args("common.config_error", &[("error", error)]);
        children.push(paragraph(ui, &page.theme, &text, None)?);
    }
    children.extend(super::page_children(ui, page, page.settings.page)?);
    children.extend(super::help::help(ui, page)?);
    let side = LengthPercentage::length(page.px(padding(look)));
    let top = match look {
        Look::Plain => TITLEBAR + 12.0,
        Look::Mac => TOOLBAR + 4.0,
        Look::Fluent => 4.0,
    };
    let column = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            gap: page.gap(10.0),
            size: Size {
                width: Dimension::percent(1.0),
                height: Dimension::auto(),
            },
            max_size: Size {
                width: max_width(look).map_or(LengthPercentageAuto::auto(), |width| {
                    LengthPercentageAuto::length(page.px(width + padding(look) * 2.0))
                }),
                height: LengthPercentageAuto::auto(),
            },
            padding: Rect {
                left: side,
                right: side,
                top: LengthPercentage::length(page.px(top)),
                bottom: LengthPercentage::length(page.px(32.0)),
            },
            ..Style::default()
        },
        Visual::Group,
        &children,
    )?;
    let column = if look == Look::Mac {
        ui.scene().node(
            Style {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::CENTER,
                ..Style::default()
            },
            Visual::Group,
            &[column],
        )?
    } else {
        column
    };
    // 每页各记各的滚动位置
    let mut scroll = ScrollArea::new(Style {
        flex_grow: 1.0,
        ..Style::default()
    });
    if look == Look::Mac {
        scroll = scroll.obscured_top(TOOLBAR);
    }
    let scroll = scroll.build(ui, &page.theme, ("content", page.settings.page), column)?;
    if look != Look::Mac {
        return Ok(scroll);
    }
    // 系统设置的工具栏浮在内容上：往下滚时内容从它底下穿过，下沿渐隐，不是一刀切齐
    let toolbar = toolbar(ui, page)?;
    ui.scene().node(
        Style {
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            min_size: Size {
                width: LengthPercentageAuto::length(0.0),
                height: LengthPercentageAuto::auto(),
            },
            ..Style::default()
        },
        Visual::Group,
        &[scroll, toolbar],
    )
}

/// macOS 的工具栏：前后箭头的胶囊加当前页名；不写应用名。
fn toolbar(ui: &mut Ui<Message>, page: &Page) -> Result<NodeId, RenderError> {
    let settings = page.settings;
    let back = arrow(
        ui,
        page,
        Glyph::Back,
        (!settings.back.is_empty()).then_some(Message::Back),
    )?;
    let forward = arrow(
        ui,
        page,
        Glyph::Forward,
        (!settings.forward.is_empty()).then_some(Message::Forward),
    )?;
    let colors = &page.theme.colors;
    let capsule = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            flex_shrink: 0.0,
            ..Style::default()
        },
        Visual::Box(BoxPaint {
            fill: Some(Fill::Solid(colors.button)),
            border: Some((page.px(0.5).max(1.0), colors.line)),
            radius: page.px(14.0),
        }),
        &[back, forward],
    )?;
    let title = text(ui, page, settings.page.label(), 15.0, 700)?;
    let window = page.palette.window;
    let (height, fade) = (page.px(TOOLBAR), page.px(TOOLBAR + TOOLBAR_FADE));
    ui.scene().node(
        Style {
            position: Position::Absolute,
            inset: Rect {
                left: LengthPercentageAuto::length(0.0),
                right: LengthPercentageAuto::length(0.0),
                top: LengthPercentageAuto::length(0.0),
                bottom: LengthPercentageAuto::auto(),
            },
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            gap: page.gap(10.0),
            size: Size {
                width: Dimension::auto(),
                height: Dimension::length(fade),
            },
            padding: Rect {
                left: LengthPercentage::length(page.px(16.0)),
                right: LengthPercentage::length(page.px(16.0)),
                top: LengthPercentage::length(0.0),
                bottom: LengthPercentage::length(fade - height),
            },
            ..Style::default()
        },
        Visual::Box(BoxPaint {
            fill: Some(Fill::Linear {
                angle: 180.0,
                stops: vec![
                    (window, 0.0),
                    (window, height / fade),
                    (window.transparent(), 1.0),
                ],
            }),
            border: None,
            radius: 0.0,
        }),
        &[capsule, title],
    )
}

/// 胶囊里的一个箭头；没得翻（`message` 为空）时变淡、点了没反应。
fn arrow(
    ui: &mut Ui<Message>,
    page: &Page,
    glyph: Glyph,
    message: Option<Message>,
) -> Result<NodeId, RenderError> {
    let secondary = page.theme.colors.secondary;
    let color = if message.is_some() {
        secondary
    } else {
        bamboo_slip::Color {
            a: secondary.a / 3,
            ..secondary
        }
    };
    let (side, size) = (page.px(28.0), page.px(14.0));
    let inset = LengthPercentageAuto::length((side - size) / 2.0);
    let icon = ui.scene().node(
        Style {
            size: Size {
                width: Dimension::length(size),
                height: Dimension::length(size),
            },
            margin: Rect {
                left: inset,
                top: inset,
                right: LengthPercentageAuto::length(0.0),
                bottom: LengthPercentageAuto::length(0.0),
            },
            ..Style::default()
        },
        Visual::Icon {
            icon: Arc::new(glyph),
            size,
            color,
        },
        &[],
    )?;
    let node = ui.scene().node(
        Style {
            size: Size {
                width: Dimension::length(side),
                height: Dimension::length(side),
            },
            ..Style::default()
        },
        Visual::Group,
        &[icon],
    )?;
    let key = ui.key(("toolbar", format!("{glyph:?}")));
    let interact = match message {
        Some(message) => Interact::new().on_click(message),
        None => Interact::new(),
    };
    ui.interact(node, key, interact);
    Ok(node)
}

fn text(
    ui: &mut Ui<Message>,
    page: &Page,
    text: &str,
    size: f32,
    weight: u16,
) -> Result<NodeId, RenderError> {
    ui.scene().node(
        Style::default(),
        Visual::Text {
            text: text.to_owned(),
            style: page.theme.text(size, weight, page.theme.colors.text),
        },
        &[],
    )
}
