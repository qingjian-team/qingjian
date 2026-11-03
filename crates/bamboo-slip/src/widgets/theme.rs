//! 控件主题：倍数、配色、字号、圆角与动效。品牌外观就是换一个主题。

use std::time::Duration;

use super::WidgetColors;
use crate::animation::Easing;
use crate::color::Color;
use crate::text::TextStyle;
use crate::text::font::{FontSpec, FontWeight};

#[derive(Debug, Clone, PartialEq)]
pub struct WidgetTheme {
    /// 点 → 像素。
    pub scale: f32,

    pub colors: WidgetColors,

    /// 正文字号（点）与行高，字族、字重在这里定。
    pub font: FontSpec,

    /// 小字（分组标题、说明）的字号（点）。
    pub small: f32,

    /// 覆盖率 gamma，见 [`TextStyle::gamma`]。
    pub gamma: f32,

    /// 控件圆角（点）；卡片按它的 1.5 倍。
    pub radius: f32,

    /// 弹出菜单的圆角（点）。
    pub popup_radius: f32,

    /// 弹出菜单出现时淡入滑入的时长；为零不播。
    pub popup_motion: Duration,

    /// 滚动条平时的粗细（点）；比 6 点细时，指针放上去才变粗。
    pub scrollbar_idle: f32,

    /// 输入框聚焦时只在底边画一条强调色（Fluent），不描整圈。
    pub input_underline: bool,

    /// 控件高度（点）：按钮、分段、输入框一样高，排在一行里对得齐。
    pub control_height: f32,

    /// 悬停、按下这类颜色变化的时长。
    pub hover: Duration,

    /// 开关拨动、选中项滑动的时长与曲线。
    pub flip: Duration,

    pub flip_easing: Easing,
}

impl WidgetTheme {
    pub fn new(colors: WidgetColors, scale: f32) -> Self {
        Self {
            scale,
            colors,
            font: FontSpec::new(13.0, 18.0),
            small: 11.0,
            gamma: 1.0,
            radius: 7.0,
            popup_radius: 7.0,
            popup_motion: Duration::ZERO,
            scrollbar_idle: 6.0,
            input_underline: false,
            control_height: 24.0,
            hover: Duration::from_millis(120),
            flip: Duration::from_millis(240),
            flip_easing: Easing::Bezier([0.3, 1.2, 0.5, 1.0]),
        }
    }

    pub fn px(&self, points: f32) -> f32 {
        points * self.scale
    }

    /// 正文字族下 `points` 大、`weight` 粗的文字样式（像素）。
    pub fn text(&self, points: f32, weight: u16, color: Color) -> TextStyle {
        let font = FontSpec {
            size: points,
            line_height: (points * self.font.line_height / self.font.size).round(),
            weight: FontWeight(weight),
            ..self.font
        };
        TextStyle::new(font.scaled(self.scale), points, color, self.gamma)
    }
}
