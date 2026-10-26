//! 外观：内置主题的列表、按主题推出的键盘配色、设置页的主题预览，以及设置里的字号档换成渲染器的字号。
//! 主题与推导规则都在 `qingjian-render`，这里只把它们交给壳（见 `docs/notes/android.md`「外观」）。

mod entry;
mod palette;
mod preview;

use qingjian_render::{Color, TextSizes, Theme};

pub use self::entry::ThemeEntry;
pub use self::palette::KeyboardPalette;
pub use self::preview::preview;

/// 全部内置主题，按 PC 设置程序列出的顺序；第一个是缺省主题。
pub fn themes() -> Vec<ThemeEntry> {
    Theme::builtins().iter().map(ThemeEntry::new).collect()
}

/// 按 id 取内置主题并换成对应外观；没有这个 id（比如以后删掉的主题）退回缺省主题。主题锁定了外观时不跟 `dark`。
pub fn theme(id: &str, dark: bool) -> Theme {
    Theme::builtin(id, dark).unwrap_or_else(|| {
        tracing::warn!(id, "没有这个主题，用缺省主题");
        Theme::light().with_dark(dark)
    })
}

/// 主题在 `dark` 外观下的键盘配色。
pub fn palette(id: &str, dark: bool) -> KeyboardPalette {
    KeyboardPalette::new(&theme(id, dark))
}

/// 字号档（候选字与译文一起的倍数）换成盖到主题上的字号；1 倍用主题自己的。
pub fn text_sizes(theme: &Theme, scale: f32) -> TextSizes {
    if !(scale.is_finite() && scale > 0.0) || (scale - 1.0).abs() < f32::EPSILON {
        return TextSizes::default();
    }
    let size = |name: &str| theme.theme_size(name).map_or(0.0, |size| size * scale);
    TextSizes::new(size("candidate"), size("annotation"))
}

/// Android 的颜色整数：A R G B 各一个字节。
pub fn argb(color: Color) -> u32 {
    u32::from_be_bytes([color.a, color.r, color.g, color.b])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_every_builtin_theme_with_the_default_first() {
        let themes = themes();
        assert_eq!(themes.len(), Theme::builtins().len());
        assert_eq!(themes[0].id, "qingjian");
        assert!(themes.iter().any(|theme| theme.id == "sakura"));
    }

    #[test]
    fn unknown_theme_falls_back_to_the_default() {
        assert_eq!(theme("nope", false).id(), "qingjian");
        assert_eq!(theme("wechat", true).id(), "wechat");
    }

    #[test]
    fn text_scale_multiplies_theme_sizes() {
        let theme = Theme::light();
        assert_eq!(text_sizes(&theme, 1.0), TextSizes::default());
        assert_eq!(text_sizes(&theme, f32::NAN), TextSizes::default());
        let sizes = text_sizes(&theme, 1.25);
        assert_eq!(sizes.candidate, Some(16.0 * 1.25));
        assert_eq!(sizes.annotation, Some(12.0 * 1.25));
    }

    #[test]
    fn locked_light_themes_stay_light_in_dark_mode() {
        assert!(!palette("sakura", true).dark);
        assert!(palette("qingjian", true).dark);
    }
}
