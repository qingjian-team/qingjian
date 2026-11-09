//! 交给壳的键盘配色（JSON 过 JNI），颜色是 Android 的 ARGB 整数。

use qingjian_render::{KeyboardColors, Theme};
use serde::Serialize;

use super::argb;

/// 字段同 [`KeyboardColors`]；壳的 `Palette` 照着读。
#[derive(Debug, Serialize)]
pub struct KeyboardPalette {
    /// 键面是深色：候选条按深色画，图标与气泡跟着用深色那一套。
    pub dark: bool,

    pub board: u32,

    pub key: u32,

    pub function_key: u32,

    pub pressed: u32,

    pub brand: u32,

    pub on_brand: u32,

    pub shadow: u32,

    pub text: u32,

    pub gloss: u32,

    pub faint: u32,

    pub fresh: u32,

    pub cloud: u32,
}

impl KeyboardPalette {
    pub fn new(theme: &Theme) -> Self {
        let colors = KeyboardColors::of(theme);
        Self {
            dark: colors.dark,
            board: argb(colors.board),
            key: argb(colors.key),
            function_key: argb(colors.function_key),
            pressed: argb(colors.pressed),
            brand: argb(colors.brand),
            on_brand: argb(colors.on_brand),
            shadow: argb(colors.shadow),
            text: argb(colors.text),
            gloss: argb(colors.gloss),
            faint: argb(colors.faint),
            fresh: argb(colors.fresh),
            cloud: argb(colors.cloud),
        }
    }
}
