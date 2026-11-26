//! Windows 11 设置的左侧导航：搜索框在最上，各页一行（Fluent 彩色图标），选中项左边一段强调色竖条；
//! 底部分隔线下是界面语言与「关于」。搜索结果弹在搜索框下面（[`suggestions`]）。标题栏用系统的（图标加窗口标题），这里不画。

use std::sync::{Arc, OnceLock};

use bamboo_slip::taffy::{
    Dimension, FlexDirection, LengthPercentage, LengthPercentageAuto, NodeId, Rect, Size, Style,
};
use bamboo_slip::{BoxPaint, Fill, RenderError, SvgImage, Ui, Visual};

use super::{language, logo, navigation, search_field, suggestions};
use crate::message::Message;
use crate::page_id::PageId;
use crate::view::Page;

/// 导航宽度（点）：WinUI 的 NavigationView 展开时是固定宽，不随窗口变；配 [`super::super::look::window_size`] 的缺省窗口约占三成。
pub const WIDTH: f32 = 288.0;

/// 导航左右的内边距（点）：照 Win11 设置，选中块离窗口左边 12 点，右边留 4 点，再加内容区的边距到正文卡片约 28 点。
const SIDE: (f32, f32) = (12.0, 4.0);

/// 图标边长（点），SVG 画在 20 × 20 的网格上。
const ICON: f32 = 20.0;

pub fn sidebar(ui: &mut Ui<Message>, page: &Page) -> Result<NodeId, RenderError> {
    let search = search_field(ui, page)?;
    // 搜索框与导航项同宽，左右边对齐
    let search = inset(ui, page, search, (4.0, 0.0, 12.0))?;
    // 搜索结果是搜索框下的浮层，导航本身不变
    suggestions(ui, page)?;
    let pages: Vec<PageId> = PageId::ALL
        .into_iter()
        .filter(|id| *id != PageId::About)
        .collect();
    let items = navigation(ui, page, &pages)?;
    let list = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            gap: page.gap(4.0),
            flex_grow: 1.0,
            ..Style::default()
        },
        Visual::Group,
        &items,
    )?;
    let line = ui.scene().node(
        Style {
            size: Size {
                width: Dimension::auto(),
                height: Dimension::length(page.px(1.0)),
            },
            margin: Rect {
                left: LengthPercentageAuto::length(page.px(4.0)),
                right: LengthPercentageAuto::length(page.px(4.0)),
                top: LengthPercentageAuto::length(page.px(4.0)),
                bottom: LengthPercentageAuto::length(page.px(4.0)),
            },
            ..Style::default()
        },
        Visual::solid(page.theme.colors.line, 0.0),
        &[],
    )?;
    let language = language(ui, page)?;
    let mut bottom = vec![line, language];
    bottom.extend(navigation(ui, page, &[PageId::About])?);
    let bottom = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            gap: page.gap(4.0),
            ..Style::default()
        },
        Visual::Group,
        &bottom,
    )?;
    ui.scene().node(
        Style {
            size: Size {
                width: Dimension::length(page.px(WIDTH)),
                height: Dimension::auto(),
            },
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            padding: Rect {
                left: LengthPercentage::length(page.px(SIDE.0)),
                right: LengthPercentage::length(page.px(SIDE.1)),
                top: LengthPercentage::length(0.0),
                bottom: LengthPercentage::length(page.px(12.0)),
            },
            ..Style::default()
        },
        Visual::Group,
        &[search, list, bottom],
    )
}

/// 这页的 Fluent 彩色图标；「关于」用青简的 logo。
pub fn icon(ui: &mut Ui<Message>, page: &Page, id: PageId) -> Result<NodeId, RenderError> {
    if id == PageId::About {
        return logo(ui, page, ICON, ICON);
    }
    let side = page.px(ICON);
    let image = Arc::clone(&icons()[index(id)]);
    let natural = image.size().0;
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
            fill: Some(Fill::Svg {
                image,
                slice: None,
                px_per_unit: side / natural,
            }),
            border: None,
            radius: 0.0,
        }),
        &[],
    )
}

fn index(id: PageId) -> usize {
    PageId::ALL
        .iter()
        .position(|page| *page == id)
        .unwrap_or_default()
}

/// 各页图标按 [`PageId::ALL`] 的顺序（「关于」除外，它在最后），第一次用时解析一次。
fn icons() -> &'static [Arc<SvgImage>] {
    static ICONS: OnceLock<Vec<Arc<SvgImage>>> = OnceLock::new();
    ICONS.get_or_init(|| {
        let sources: [&[u8]; 12] = [
            include_bytes!("../../../assets/nav/input.svg"),
            include_bytes!("../../../assets/nav/candidates.svg"),
            include_bytes!("../../../assets/nav/language.svg"),
            include_bytes!("../../../assets/nav/appearance.svg"),
            include_bytes!("../../../assets/nav/shortcuts.svg"),
            include_bytes!("../../../assets/nav/phrases.svg"),
            include_bytes!("../../../assets/nav/dictionaries.svg"),
            include_bytes!("../../../assets/nav/cloud.svg"),
            include_bytes!("../../../assets/nav/stats.svg"),
            include_bytes!("../../../assets/nav/vocabulary.svg"),
            include_bytes!("../../../assets/nav/advanced.svg"),
            include_bytes!("../../../assets/nav/experimental.svg"),
        ];
        sources
            .iter()
            .map(|data| Arc::new(SvgImage::parse(data).expect("bundled nav icon is valid SVG")))
            .collect()
    })
}

/// 给 `node` 包一层外边距（上、左右、下，点）。
fn inset(
    ui: &mut Ui<Message>,
    page: &Page,
    node: NodeId,
    (top, side, bottom): (f32, f32, f32),
) -> Result<NodeId, RenderError> {
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            padding: Rect {
                left: LengthPercentage::length(page.px(side)),
                right: LengthPercentage::length(page.px(side)),
                top: LengthPercentage::length(page.px(top)),
                bottom: LengthPercentage::length(page.px(bottom)),
            },
            ..Style::default()
        },
        Visual::Group,
        &[node],
    )
}
