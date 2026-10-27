//! 生词本页两个视图共用的小件：横排一行、一段文字、带底色的小标签、翻页。

use bamboo_slip::taffy::{
    AlignItems, FlexDirection, FlexWrap, LengthPercentage, NodeId, Rect, Style,
};
use bamboo_slip::{BoxPaint, Button, Color, Fill, RenderError, SettingRow, Ui, Visual};

use crate::message::Message;
use crate::view::Page;

pub fn row(
    ui: &mut Ui<Message>,
    page: &Page,
    gap: f32,
    children: &[NodeId],
) -> Result<NodeId, RenderError> {
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            align_items: AlignItems::BASELINE,
            gap: page.gap(gap),
            ..Style::default()
        },
        Visual::Group,
        children,
    )
}

pub fn text(
    ui: &mut Ui<Message>,
    page: &Page,
    text: &str,
    points: f32,
    weight: u16,
    color: Color,
) -> Result<NodeId, RenderError> {
    ui.scene().node(
        Style::default(),
        Visual::Text {
            text: text.to_owned(),
            style: page.theme.text(points, weight, color),
        },
        &[],
    )
}

/// 「生词」「认识了」这样的小标签：`accent` 用强调色，否则灰的。
pub fn tag(
    ui: &mut Ui<Message>,
    page: &Page,
    label: &str,
    accent: bool,
) -> Result<NodeId, RenderError> {
    let colors = &page.theme.colors;
    let (fill, color) = if accent {
        (colors.surface.lerp(colors.accent, 0.16), colors.accent)
    } else {
        (colors.well, colors.secondary)
    };
    let label = text(ui, page, label, page.theme.small, 600, color)?;
    let (x, y) = (
        LengthPercentage::length(page.px(7.0)),
        LengthPercentage::length(page.px(2.0)),
    );
    ui.scene().node(
        Style {
            padding: Rect {
                left: x,
                right: x,
                top: y,
                bottom: y,
            },
            align_self: Some(AlignItems::CENTER),
            ..Style::default()
        },
        Visual::Box(BoxPaint {
            fill: Some(Fill::Solid(fill)),
            border: None,
            radius: page.px(4.0),
        }),
        &[label],
    )
}

/// 「第 2 / 5 页」与上一页、下一页。
pub fn pager(
    ui: &mut Ui<Message>,
    page: &Page,
    current: usize,
    pages: usize,
) -> Result<NodeId, RenderError> {
    let theme = &page.theme;
    let previous = Button::new("上一页").disabled(current == 0).build(
        ui,
        theme,
        "vocabulary-previous",
        Message::VocabularyPage(current.saturating_sub(1)),
    )?;
    let next = Button::new("下一页").disabled(current + 1 >= pages).build(
        ui,
        theme,
        "vocabulary-next",
        Message::VocabularyPage(current + 1),
    )?;
    let buttons = row(ui, page, 8.0, &[previous, next])?;
    SettingRow::new(&format!("第 {} / {} 页", current + 1, pages))
        .control(buttons)
        .build(ui, theme)
}
