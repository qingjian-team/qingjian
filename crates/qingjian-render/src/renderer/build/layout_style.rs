//! 主题里的盒子属性 → Taffy 样式。长度进来是点，乘倍数成像素。

use taffy::{
    AlignItems, Dimension, Display, FlexDirection, LengthPercentage, LengthPercentageAuto,
    Position, Rect, Size, Style,
};

use crate::theme::file::node::{
    Align, BoxSpec, Direction, Edges, Length, Position as SpecPosition,
};

/// 盒子属性（不含排列方向）。
pub(super) fn from_box(spec: &BoxSpec, scale: f32) -> Style {
    let px = |points: f32| points * scale;
    let mut style = Style::default();
    if let Some(margin) = spec.margin {
        style.margin = edges_auto(margin, scale);
    }
    if let Some(padding) = spec.padding {
        style.padding = edges(padding, scale);
    }
    if let Some(gap) = spec.gap {
        style.gap = Size {
            width: LengthPercentage::length(px(gap)),
            height: LengthPercentage::length(px(gap)),
        };
    }
    if let Some(width) = spec.width {
        style.size.width = Dimension::length(px(width));
    }
    if let Some(height) = spec.height {
        style.size.height = Dimension::length(px(height));
    }
    if let Some(min_width) = spec.min_width {
        style.min_size.width = LengthPercentageAuto::length(px(min_width));
    }
    if let Some(SpecPosition::Absolute) = spec.position {
        style.position = Position::Absolute;
    }
    if let Some(inset) = spec.inset {
        style.inset = edges_auto(inset, scale);
    }
    style.align_self = spec.align_self.map(align);
    style
}

/// 框：按方向排子节点，交叉轴缺省顶端（左端）对齐。
pub(super) fn flex(style: Style, direction: Direction) -> Style {
    Style {
        display: Display::Flex,
        flex_direction: match direction {
            Direction::Row => FlexDirection::Row,
            Direction::Column => FlexDirection::Column,
        },
        align_items: Some(AlignItems::START),
        ..style
    }
}

fn align(value: Align) -> AlignItems {
    match value {
        Align::Start => AlignItems::START,
        Align::End => AlignItems::END,
        Align::Center => AlignItems::CENTER,
        Align::Stretch => AlignItems::STRETCH,
    }
}

fn edges_auto(value: Edges, scale: f32) -> Rect<LengthPercentageAuto> {
    let side = |length: Length| match length {
        Length::Points(points) => LengthPercentageAuto::length(points * scale),
        Length::Auto => LengthPercentageAuto::auto(),
    };
    Rect {
        left: side(value.left),
        right: side(value.right),
        top: side(value.top),
        bottom: side(value.bottom),
    }
}

/// 内边距不认 `auto`，当 0。
fn edges(value: Edges, scale: f32) -> Rect<LengthPercentage> {
    let side = |length: Length| match length {
        Length::Points(points) => LengthPercentage::length(points * scale),
        Length::Auto => LengthPercentage::length(0.0),
    };
    Rect {
        left: side(value.left),
        right: side(value.right),
        top: side(value.top),
        bottom: side(value.bottom),
    }
}
