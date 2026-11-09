//! Fluent（Windows 11）风格的主题预设，数值取自 WinUI 的设计令牌。可选用，引擎的缺省主题不受影响。

use std::time::Duration;

use super::{WidgetColors, WidgetTheme};
use crate::animation::Easing;
use crate::appearance::Accent;
use crate::color::Color;
use crate::text::font::FontSpec;

impl WidgetColors {
    pub const fn fluent(dark: bool, accent: Accent) -> Self {
        if dark {
            Self {
                text: Color::rgb(0xFF, 0xFF, 0xFF),
                secondary: Color::gray(255, 200),
                accent: accent.on_dark,
                // 深色下的强调色很亮，上面放黑字
                on_accent: Color::rgb(0, 0, 0),
                well: Color::gray(255, 15),
                well_hover: Color::gray(255, 21),
                chip: Color::rgb(0x3A, 0x3A, 0x3A),
                chip_text: Color::rgb(0xFF, 0xFF, 0xFF),
                surface: Color::rgb(0x2B, 0x2B, 0x2B),
                popup: Color::rgb(0x2C, 0x2C, 0x2C),
                line: Color::gray(255, 21),
                hover: Color::gray(255, 15),
                pressed: Color::gray(255, 10),
                selected: Color::gray(255, 15),
                selected_text: Color::rgb(0xFF, 0xFF, 0xFF),
                focus: Color::rgb(0xFF, 0xFF, 0xFF),
                knob: Color::rgb(0, 0, 0),
                button: Color::gray(255, 15),
                shadow: Color::gray(0, 90),
            }
        } else {
            Self {
                text: Color::gray(0, 228),
                secondary: Color::gray(0, 155),
                accent: accent.on_light,
                on_accent: Color::rgb(0xFF, 0xFF, 0xFF),
                well: Color::gray(0, 10),
                well_hover: Color::gray(0, 16),
                chip: Color::rgb(0xFF, 0xFF, 0xFF),
                chip_text: Color::gray(0, 228),
                surface: Color::rgb(0xFB, 0xFB, 0xFB),
                popup: Color::rgb(0xF9, 0xF9, 0xF9),
                line: Color::gray(0, 20),
                hover: Color::gray(0, 10),
                pressed: Color::gray(0, 6),
                selected: Color::gray(0, 10),
                selected_text: Color::gray(0, 228),
                focus: Color::gray(0, 228),
                knob: Color::rgb(0xFF, 0xFF, 0xFF),
                button: Color::rgb(0xFD, 0xFD, 0xFD),
                shadow: Color::gray(0, 36),
            }
        }
    }

    /// 窗口底色（没有系统材质时）。
    pub const fn fluent_backdrop(dark: bool) -> Color {
        if dark {
            Color::rgb(0x20, 0x20, 0x20)
        } else {
            Color::rgb(0xF3, 0xF3, 0xF3)
        }
    }
}

impl WidgetTheme {
    pub fn fluent(colors: WidgetColors, scale: f32) -> Self {
        Self {
            font: FontSpec::new(14.0, 20.0),
            small: 12.0,
            radius: 4.0,
            popup_radius: 8.0,
            popup_motion: Duration::from_millis(167),
            scrollbar_idle: 2.0,
            input_underline: true,
            control_height: 32.0,
            hover: Duration::from_millis(83),
            flip: Duration::from_millis(167),
            flip_easing: Easing::Bezier([0.0, 0.0, 0.0, 1.0]),
            ..Self::new(colors, scale)
        }
    }
}
