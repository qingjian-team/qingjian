//! 面板配色：控件与窗口用 bamboo-slip 的 Fluent 预设，这里只放剪贴板自己的类型色与代码高亮色。

mod code_colors;

use bamboo_slip::{Accent, Color, WidgetColors};
use qingjian_clipboard::ClipKind;

use code_colors::CodeColors;

/// 亚克力上那层主题色的不透明度。
const ACRYLIC_TINT: u8 = 200;

pub struct Palette {
    pub widgets: WidgetColors,

    pub window: Color,

    pub code: CodeColors,
}

impl Palette {
    /// `acrylic`：窗口背后有系统材质，底色压一层半透明的主题色，透出模糊又不随壁纸发灰。
    pub fn new(dark: bool, accent: Accent, acrylic: bool) -> Self {
        let window = WidgetColors::fluent_backdrop(dark);
        Self {
            widgets: WidgetColors::fluent(dark, accent),
            window: if acrylic {
                Color {
                    a: ACRYLIC_TINT,
                    ..window
                }
            } else {
                window
            },
            code: if dark {
                CodeColors {
                    keyword: Color::rgb(0xC5, 0x8A, 0xF9),
                    string: Color::rgb(0x8F, 0xCB, 0x9B),
                    comment: Color::rgb(0x9A, 0x9A, 0x9A),
                    number: Color::rgb(0xE8, 0xA8, 0x70),
                }
            } else {
                CodeColors {
                    keyword: Color::rgb(0x77, 0x3A, 0xB5),
                    string: Color::rgb(0x0F, 0x7B, 0x0F),
                    comment: Color::rgb(0x6E, 0x6E, 0x6E),
                    number: Color::rgb(0xA8, 0x54, 0x12),
                }
            },
        }
    }
}

/// 卡片标题栏的颜色，上面放白字：饱和度压低，深色外观再压暗一档，不抢内容。
pub fn kind_color(kind: ClipKind, dark: bool) -> Color {
    match (kind, dark) {
        (ClipKind::Text, false) => Color::rgb(0x3F, 0x6F, 0x52),
        (ClipKind::Text, true) => Color::rgb(0x2F, 0x52, 0x40),
        (ClipKind::Rich, false) => Color::rgb(0x2F, 0x70, 0x72),
        (ClipKind::Rich, true) => Color::rgb(0x27, 0x51, 0x4F),
        (ClipKind::Code, false) => Color::rgb(0x5D, 0x5A, 0x96),
        (ClipKind::Code, true) => Color::rgb(0x45, 0x43, 0x70),
        (ClipKind::Image, false) => Color::rgb(0x96, 0x60, 0x3A),
        (ClipKind::Image, true) => Color::rgb(0x6B, 0x4A, 0x2F),
        (ClipKind::Files, false) => Color::rgb(0x3D, 0x6A, 0x99),
        (ClipKind::Files, true) => Color::rgb(0x2F, 0x4E, 0x6E),
    }
}
