//! 控件共用的小件：文字节点、焦点环、常用的布局样式与投影。

use taffy::{
    AlignItems, Dimension, FlexDirection, JustifyContent, LengthPercentage, LengthPercentageAuto,
    NodeId, Position, Rect, Size, Style,
};

use super::WidgetTheme;
use crate::color::Color;
use crate::error::RenderError;
use crate::scene::{BoxPaint, Effect, EffectKind, Fill, Visual};
use crate::ui::Ui;

/// 焦点环离控件外沿的距离（点）。
const RING_GAP: f32 = 3.0;

pub(super) fn label<M>(
    ui: &mut Ui<M>,
    theme: &WidgetTheme,
    text: &str,
    points: f32,
    weight: u16,
    color: Color,
) -> Result<NodeId, RenderError> {
    ui.scene().node(
        Style::default(),
        Visual::Text {
            text: text.to_owned(),
            style: theme.text(points, weight, color),
        },
        &[],
    )
}

/// 外圈焦点环：盖在控件外面一圈，`radius` 是控件自己的圆角（像素）。
pub(super) fn focus_ring<M>(
    ui: &mut Ui<M>,
    theme: &WidgetTheme,
    radius: f32,
) -> Result<NodeId, RenderError> {
    let gap = theme.px(RING_GAP);
    ui.scene().node(
        Style {
            position: Position::Absolute,
            inset: uniform_inset(-gap),
            ..Style::default()
        },
        Visual::Box(BoxPaint {
            fill: None,
            border: Some((theme.px(2.0), theme.colors.focus)),
            radius: radius + gap,
        }),
        &[],
    )
}

/// 铺满父节点的绝对定位层（选中项的底、按下时的遮罩）。
pub(super) fn overlay() -> Style {
    Style {
        position: Position::Absolute,
        inset: uniform_inset(0.0),
        ..Style::default()
    }
}

fn uniform_inset(value: f32) -> Rect<LengthPercentageAuto> {
    let value = LengthPercentageAuto::length(value);
    Rect {
        left: value,
        right: value,
        top: value,
        bottom: value,
    }
}

pub(super) fn size(width: Option<f32>, height: Option<f32>) -> Size<Dimension> {
    Size {
        width: width.map_or(Dimension::auto(), Dimension::length),
        height: height.map_or(Dimension::auto(), Dimension::length),
    }
}

/// 上下、左右的内边距（点）。
pub(super) fn padding(
    theme: &WidgetTheme,
    vertical: f32,
    horizontal: f32,
) -> Rect<LengthPercentage> {
    let (v, h) = (
        LengthPercentage::length(theme.px(vertical)),
        LengthPercentage::length(theme.px(horizontal)),
    );
    Rect {
        left: h,
        right: h,
        top: v,
        bottom: v,
    }
}

pub(super) fn gap(theme: &WidgetTheme, points: f32) -> Size<LengthPercentage> {
    let value = LengthPercentage::length(theme.px(points));
    Size {
        width: value,
        height: value,
    }
}

/// 横排、垂直居中。
pub(super) fn row(theme: &WidgetTheme, spacing: f32) -> Style {
    Style {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::CENTER,
        gap: gap(theme, spacing),
        ..Style::default()
    }
}

pub(super) fn column(theme: &WidgetTheme, spacing: f32) -> Style {
    Style {
        flex_direction: FlexDirection::Column,
        gap: gap(theme, spacing),
        ..Style::default()
    }
}

/// 子节点水平垂直都居中。
pub(super) fn centered(width: f32, height: f32) -> Style {
    Style {
        size: size(Some(width), Some(height)),
        justify_content: JustifyContent::CENTER,
        align_items: AlignItems::CENTER,
        flex_shrink: 0.0,
        ..Style::default()
    }
}

pub(super) fn solid(fill: Option<Color>, border: Option<(f32, Color)>, radius: f32) -> Visual {
    Visual::Box(BoxPaint {
        fill: fill.map(Fill::Solid),
        border,
        radius,
    })
}

/// 浮起的小件（旋钮、选中项）下面的投影。
pub(super) fn lift(theme: &WidgetTheme) -> Effect {
    Effect {
        kind: EffectKind::DropShadow,
        x: 0.0,
        y: theme.px(1.0),
        blur: theme.px(2.0),
        spread: 0.0,
        color: theme.colors.shadow,
    }
}

/// `top` 盖在 `base` 上的颜色（不透明度按正常混合叠）：悬停、按下的底是半透明的一层，叠在控件原本的底色上。
pub(super) fn over(base: Color, top: Color) -> Color {
    let (ta, ba) = (f32::from(top.a) / 255.0, f32::from(base.a) / 255.0);
    let a = ta + ba * (1.0 - ta);
    if a <= 0.0 {
        return base.transparent();
    }
    let channel = |t: u8, b: u8| {
        ((f32::from(t) * ta + f32::from(b) * ba * (1.0 - ta)) / a)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color::rgba(
        channel(top.r, base.r),
        channel(top.g, base.g),
        channel(top.b, base.b),
        (a * 255.0).round() as u8,
    )
}

/// 禁用的控件整体淡化。
pub(super) const DISABLED_OPACITY: f32 = 0.4;
