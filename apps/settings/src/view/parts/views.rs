//! 切换同一页几种视图的那一行（统计「输入 / 云服务」、生词本「时间线 / 词表」）：
//! 竹青靠左的分段控件，macOS 居中的分段控件（系统设置「电池」页那样），Windows 靠左的 SelectorBar；
//! `trailing`（生词本的语言）放在这一行最右边，竹青紧跟在分段控件后面。

use bamboo_slip::taffy::{
    AlignItems, Dimension, FlexDirection, FlexWrap, JustifyContent, LengthPercentageAuto, NodeId,
    Rect, Style,
};
use bamboo_slip::{Look, RenderError, Segmented, SelectorBar, Ui, Visual};

use crate::message::Message;
use crate::view::Page;

pub fn view_switch(
    ui: &mut Ui<Message>,
    page: &Page,
    local: &'static str,
    (labels, selected): (&[&str], usize),
    trailing: Option<NodeId>,
    on_select: impl Fn(usize) -> Message,
) -> Result<NodeId, RenderError> {
    let theme = &page.theme;
    if theme.look == Look::Plain {
        let mut children =
            vec![Segmented::new(labels, selected).build(ui, theme, local, on_select)?];
        children.extend(trailing);
        return ui.scene().node(
            Style {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                gap: page.gap(12.0),
                ..Style::default()
            },
            Visual::Group,
            &children,
        );
    }
    let fluent = theme.look == Look::Fluent;
    let switch = if fluent {
        let bar = SelectorBar::new(labels, selected).build(ui, theme, local, on_select)?;
        // 选项字与下面内容的左边对齐，悬停底色伸到边外
        ui.scene().node(
            Style {
                margin: Rect {
                    left: LengthPercentageAuto::length(-page.px(12.0)),
                    ..Rect::zero()
                },
                flex_shrink: 0.0,
                ..Style::default()
            },
            Visual::Group,
            &[bar],
        )?
    } else {
        Segmented::new(labels, selected).build(ui, theme, local, on_select)?
    };
    let side = |ui: &mut Ui<Message>, children: &[NodeId]| {
        ui.scene().node(
            Style {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::FLEX_END,
                align_items: AlignItems::CENTER,
                flex_grow: 1.0,
                flex_basis: Dimension::length(0.0),
                ..Style::default()
            },
            Visual::Group,
            children,
        )
    };
    let right = side(ui, trailing.as_slice())?;
    let children = if fluent {
        vec![switch, right]
    } else {
        vec![side(ui, &[])?, switch, right]
    };
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            ..Style::default()
        },
        Visual::Group,
        &children,
    )
}
