//! 样例用的竹青配色（浅色、深色）与窗口、侧栏的底色。

use bamboo_slip::{Color, WidgetColors};

pub struct Palette {
    pub widgets: WidgetColors,

    pub window: Color,

    pub sidebar: Color,
}

pub const LIGHT: Palette = Palette {
    widgets: WidgetColors {
        text: Color::rgb(0x11, 0x22, 0x38),
        secondary: Color::rgb(0x78, 0x83, 0x95),
        accent: Color::rgb(0x33, 0x6F, 0x33),
        on_accent: Color::rgb(0xFF, 0xFF, 0xFF),
        well: Color::rgb(0xEC, 0xF1, 0xEA),
        well_hover: Color::rgb(0xDF, 0xE8, 0xDC),
        chip: Color::rgb(0xFF, 0xFF, 0xFF),
        chip_text: Color::rgb(0x33, 0x6F, 0x33),
        surface: Color::rgb(0xFF, 0xFF, 0xFF),
        popup: Color::rgb(0xFF, 0xFF, 0xFF),
        line: Color::rgb(0xE4, 0xEC, 0xE2),
        hover: Color::rgba(0x33, 0x6F, 0x33, 14),
        pressed: Color::rgba(0x33, 0x6F, 0x33, 30),
        selected: Color::rgba(0x94, 0xBE, 0x52, 46),
        selected_text: Color::rgb(0x33, 0x6F, 0x33),
        focus: Color::rgba(0x94, 0xBE, 0x52, 220),
        knob: Color::rgb(0xFF, 0xFF, 0xFF),
        button: Color::rgb(0xFF, 0xFF, 0xFF),
        shadow: Color::gray(0, 50),
    },
    window: Color::rgb(0xF7, 0xFA, 0xF7),
    sidebar: Color::rgb(0xED, 0xF3, 0xEB),
};

pub const DARK: Palette = Palette {
    widgets: WidgetColors {
        text: Color::rgb(0xE4, 0xEA, 0xE3),
        secondary: Color::rgb(0x8B, 0x97, 0x8D),
        accent: Color::rgb(0x6C, 0xA2, 0x45),
        on_accent: Color::rgb(0xFF, 0xFF, 0xFF),
        well: Color::rgba(255, 255, 255, 16),
        well_hover: Color::rgba(255, 255, 255, 28),
        chip: Color::rgb(0x3A, 0x47, 0x39),
        chip_text: Color::rgb(0xB4, 0xD8, 0x86),
        surface: Color::rgb(0x1F, 0x27, 0x20),
        popup: Color::rgb(0x25, 0x2E, 0x26),
        line: Color::rgba(255, 255, 255, 18),
        hover: Color::rgba(255, 255, 255, 12),
        pressed: Color::rgba(255, 255, 255, 24),
        selected: Color::rgba(0x94, 0xBE, 0x52, 40),
        selected_text: Color::rgb(0xB4, 0xD8, 0x86),
        focus: Color::rgba(0x94, 0xBE, 0x52, 200),
        knob: Color::rgb(0xF2, 0xF2, 0xF2),
        button: Color::rgba(255, 255, 255, 18),
        shadow: Color::gray(0, 110),
    },
    window: Color::rgb(0x15, 0x1A, 0x16),
    sidebar: Color::rgb(0x1B, 0x22, 0x1C),
};
