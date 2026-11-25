//! macOS（系统设置）风格的主题预设，数值照 macOS 26 的系统语义色与控件尺寸。可选用，引擎的缺省主题不受影响。

use std::time::Duration;

use super::theme::Look;
use super::{WidgetColors, WidgetTheme};
use crate::animation::Easing;
use crate::color::Color;
use crate::text::font::FontSpec;

impl WidgetColors {
    /// `accent` 是系统强调色（`controlAccentColor`）在当前外观下的值。
    pub const fn mac(dark: bool, accent: Color) -> Self {
        if dark {
            Self {
                text: Color::gray(255, 224),
                secondary: Color::gray(255, 140),
                accent,
                on_accent: Color::rgb(0xFF, 0xFF, 0xFF),
                well: Color::gray(255, 18),
                well_hover: Color::gray(255, 30),
                chip: Color::rgb(0x63, 0x63, 0x66),
                chip_text: Color::rgb(0xFF, 0xFF, 0xFF),
                surface: Color::gray(255, 12),
                popup: Color::rgb(0x34, 0x34, 0x38),
                line: Color::gray(255, 18),
                hover: Color::gray(255, 15),
                pressed: Color::gray(255, 26),
                selected: tint(accent, 66),
                selected_text: Color::rgb(0xFF, 0xFF, 0xFF),
                focus: tint(accent, 128),
                knob: Color::rgb(0xEE, 0xEE, 0xEE),
                button: Color::gray(255, 36),
                shadow: Color::gray(0, 110),
            }
        } else {
            Self {
                text: Color::gray(0, 219),
                secondary: Color::gray(0, 143),
                accent,
                on_accent: Color::rgb(0xFF, 0xFF, 0xFF),
                well: Color::gray(0, 15),
                well_hover: Color::gray(0, 26),
                chip: Color::rgb(0xFF, 0xFF, 0xFF),
                chip_text: Color::gray(0, 219),
                surface: Color::rgb(0xFF, 0xFF, 0xFF),
                popup: Color::rgb(0xF6, 0xF6, 0xF8),
                line: Color::gray(0, 19),
                hover: Color::gray(0, 13),
                pressed: Color::gray(0, 24),
                selected: tint(accent, 41),
                selected_text: Color::gray(0, 219),
                focus: tint(accent, 128),
                knob: Color::rgb(0xFF, 0xFF, 0xFF),
                button: Color::rgb(0xFF, 0xFF, 0xFF),
                shadow: Color::gray(0, 40),
            }
        }
    }

    /// 内容区的底色。
    pub const fn mac_backdrop(dark: bool) -> Color {
        if dark {
            Color::rgb(0x1E, 0x1E, 0x20)
        } else {
            Color::rgb(0xF4, 0xF4, 0xF6)
        }
    }

    /// 侧栏的底色（没有系统毛玻璃时）。
    pub const fn mac_sidebar(dark: bool) -> Color {
        if dark {
            Color::rgb(0x2A, 0x2A, 0x2E)
        } else {
            Color::rgb(0xE6, 0xE6, 0xEB)
        }
    }
}

impl WidgetTheme {
    pub fn mac(colors: WidgetColors, scale: f32) -> Self {
        Self {
            look: Look::Mac,
            font: FontSpec::new(13.0, 16.0),
            small: 11.0,
            radius: 6.0,
            popup_radius: 10.0,
            popup_motion: Duration::from_millis(120),
            scrollbar_idle: 3.0,
            control_height: 22.0,
            hover: Duration::from_millis(100),
            flip: Duration::from_millis(200),
            flip_easing: Easing::Bezier([0.25, 0.1, 0.25, 1.0]),
            ..Self::new(colors, scale)
        }
    }
}

/// 强调色淡成 `alpha` 的不透明度：选中、焦点这类浅底。
const fn tint(accent: Color, alpha: u8) -> Color {
    Color::rgba(accent.r, accent.g, accent.b, alpha)
}
