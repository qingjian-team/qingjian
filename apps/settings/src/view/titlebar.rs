//! Windows 11 设置风格的顶部自绘标题栏：左侧留白作拖动区，正中放胶囊圆角搜索框
//! （搜索结果浮层从它正下方弹出），右上角放贴顶的最小化 / 最大化 / 关闭三按钮；
//! 其余空白区域交给系统拖动与双击最大化。

use bamboo_slip::taffy::{
    AlignItems, Dimension, FlexDirection, JustifyContent, NodeId, Size, Style,
};
use bamboo_slip::{
    CAPTION_BAR_HEIGHT, Interact, RenderError, Ui, Visual, WindowRegion, caption_buttons,
};

use super::{Page, sidebar};
use crate::message::Message;

/// 标题栏高度（点）。
pub const HEIGHT: f32 = CAPTION_BAR_HEIGHT;

pub fn titlebar(ui: &mut Ui<Message>, page: &Page) -> Result<NodeId, RenderError> {
    // 左侧留白与右侧等权占位，让搜索框严格水平居中，同时整片左上角可直接拖动窗口
    let left = ui.scene().node(
        Style {
            flex_grow: 1.0,
            flex_shrink: 1.0,
            flex_basis: Dimension::length(0.0),
            ..Style::default()
        },
        Visual::Group,
        &[],
    )?;

    let window_width = page.window.0 / page.scale;
    let search_width = (window_width - 320.0).clamp(240.0, 480.0);
    let search_inner = sidebar::search_field(ui, page)?;
    let center = ui.scene().node(
        Style {
            size: Size {
                width: Dimension::length(page.px(search_width)),
                height: Dimension::auto(),
            },
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::CENTER,
            ..Style::default()
        },
        Visual::Group,
        &[search_inner],
    )?;
    // 搜索结果浮层按搜索框上一帧的矩形从标题栏下方弹出
    sidebar::suggestions(ui, page)?;

    let buttons = caption_buttons(ui, &page.theme)?;
    let right = ui.scene().node(
        Style {
            flex_grow: 1.0,
            flex_shrink: 1.0,
            flex_basis: Dimension::length(0.0),
            size: Size {
                width: Dimension::auto(),
                height: Dimension::percent(1.0),
            },
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FLEX_END,
            align_items: AlignItems::FLEX_START,
            ..Style::default()
        },
        Visual::Group,
        &[buttons],
    )?;

    let bar = ui.scene().node(
        Style {
            size: Size {
                width: Dimension::percent(1.0),
                height: Dimension::length(page.px(HEIGHT)),
            },
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            ..Style::default()
        },
        Visual::Group,
        &[left, center, right],
    )?;
    let key = ui.key("titlebar");
    ui.interact(bar, key, Interact::new().window_region(WindowRegion::Drag));
    Ok(bar)
}
