//! 控件的配色，浅色、深色各一套，由使用方按外观挑好传进来。

use crate::color::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WidgetColors {
    /// 正文。
    pub text: Color,

    /// 次要文字、占位符、图标。
    pub secondary: Color,

    /// 强调色：开着的开关、勾上的勾选框、主按钮、链接。
    pub accent: Color,

    /// 强调色上的文字与勾。
    pub on_accent: Color,

    /// 控件槽：分段选择的底、关着的开关、步进钮。
    pub well: Color,

    pub well_hover: Color,

    /// 分段选择选中项的底与字。
    pub chip: Color,

    pub chip_text: Color,

    /// 卡片、输入框的底。
    pub surface: Color,

    /// 弹出菜单的底：要不透明，盖住底下的内容。
    pub popup: Color,

    /// 卡片边框、分隔线、输入框边框。
    pub line: Color,

    /// 列表项、按钮悬停与按下时垫的底。
    pub hover: Color,

    pub pressed: Color,

    /// 导航项选中的底与字。
    pub selected: Color,

    pub selected_text: Color,

    /// 键盘焦点环。
    pub focus: Color,

    /// 开关的旋钮。
    pub knob: Color,

    /// 次按钮的底。
    pub button: Color,

    /// 旋钮、浮起的选中项下面的投影。
    pub shadow: Color,
}

impl WidgetColors {
    pub const LIGHT: Self = Self {
        text: Color::gray(0, 222),
        secondary: Color::gray(0, 128),
        accent: Color::rgb(0x0A, 0x84, 0xFF),
        on_accent: Color::rgb(0xFF, 0xFF, 0xFF),
        well: Color::gray(0, 16),
        well_hover: Color::gray(0, 28),
        chip: Color::rgb(0xFF, 0xFF, 0xFF),
        chip_text: Color::gray(0, 230),
        surface: Color::rgb(0xFF, 0xFF, 0xFF),
        popup: Color::rgb(0xFF, 0xFF, 0xFF),
        line: Color::gray(0, 22),
        hover: Color::gray(0, 12),
        pressed: Color::gray(0, 26),
        selected: Color::gray(0, 22),
        selected_text: Color::gray(0, 240),
        focus: Color::rgba(0x0A, 0x84, 0xFF, 150),
        knob: Color::rgb(0xFF, 0xFF, 0xFF),
        button: Color::rgb(0xFF, 0xFF, 0xFF),
        shadow: Color::gray(0, 56),
    };

    pub const DARK: Self = Self {
        text: Color::gray(255, 222),
        secondary: Color::gray(255, 140),
        accent: Color::rgb(0x0A, 0x84, 0xFF),
        on_accent: Color::rgb(0xFF, 0xFF, 0xFF),
        well: Color::gray(255, 22),
        well_hover: Color::gray(255, 34),
        chip: Color::rgb(0x5A, 0x5A, 0x5E),
        chip_text: Color::gray(255, 240),
        surface: Color::gray(255, 12),
        popup: Color::rgb(0x2A, 0x2A, 0x2D),
        line: Color::gray(255, 20),
        hover: Color::gray(255, 14),
        pressed: Color::gray(255, 30),
        selected: Color::gray(255, 26),
        selected_text: Color::gray(255, 245),
        focus: Color::rgba(0x40, 0x9C, 0xFF, 170),
        knob: Color::rgb(0xF2, 0xF2, 0xF2),
        button: Color::gray(255, 26),
        shadow: Color::gray(0, 90),
    };
}
