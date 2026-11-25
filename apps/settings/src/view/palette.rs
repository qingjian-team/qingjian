//! 设置程序的配色：控件颜色与窗口、侧栏、预览卡的底色。竹青（[`LIGHT`] / [`DARK`]）只给 [`Look::Plain`]；
//! 照系统设置画的两套跟随系统强调色，竹青只留在 logo 与图标上。

use bamboo_slip::{Accent, Color, Look, WidgetColors};

pub struct Palette {
    pub widgets: WidgetColors,

    pub window: Color,

    pub sidebar: Color,

    /// 预览卡的渐变底（左上 → 右下）。
    pub hero: [Color; 2],

    /// 主题卡片的底。
    pub tile: Color,
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
    hero: [Color::rgb(0xE8, 0xF2, 0xE1), Color::rgb(0xFF, 0xFF, 0xFF)],
    tile: Color::rgb(0xF4, 0xF8, 0xF3),
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
    hero: [Color::rgb(0x22, 0x30, 0x22), Color::rgb(0x1C, 0x22, 0x1D)],
    tile: Color::rgba(255, 255, 255, 10),
};

/// 系统读不到强调色时：macOS 的 systemBlue。
const MAC_BLUE: Color = Color::rgb(0x00, 0x7A, 0xFF);

/// 这一帧的配色：`look` 是外观，`dark` 是系统当前是不是深色。
pub fn palette(look: Look, dark: bool) -> Palette {
    match look {
        Look::Plain if dark => DARK,
        Look::Plain => LIGHT,
        Look::Mac => {
            let accent = Accent::system().map_or(MAC_BLUE, |accent| {
                if dark {
                    accent.on_dark
                } else {
                    accent.on_light
                }
            });
            Palette {
                widgets: WidgetColors::mac(dark, accent),
                window: WidgetColors::mac_backdrop(dark),
                sidebar: WidgetColors::mac_sidebar(dark),
                hero: if dark {
                    [Color::rgb(0x2C, 0x2C, 0x30), Color::rgb(0x24, 0x24, 0x26)]
                } else {
                    [Color::rgb(0xEC, 0xEE, 0xF2), Color::rgb(0xFF, 0xFF, 0xFF)]
                },
                tile: if dark {
                    Color::gray(255, 10)
                } else {
                    Color::rgb(0xF4, 0xF4, 0xF6)
                },
            }
        }
        Look::Fluent => {
            let accent = Accent::system().unwrap_or(Accent::FLUENT_BLUE);
            let window = WidgetColors::fluent_backdrop(dark);
            Palette {
                widgets: WidgetColors::fluent(dark, accent),
                window,
                sidebar: window,
                hero: if dark {
                    [Color::rgb(0x2C, 0x2C, 0x2C), Color::rgb(0x25, 0x25, 0x25)]
                } else {
                    [Color::rgb(0xEE, 0xF1, 0xF6), Color::rgb(0xFB, 0xFB, 0xFB)]
                },
                tile: if dark {
                    Color::gray(255, 10)
                } else {
                    Color::rgb(0xF6, 0xF6, 0xF6)
                },
            }
        }
    }
}
