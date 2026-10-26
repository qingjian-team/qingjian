//! 手机键盘的配色：从主题的颜色变量推出来，主题文件里不用另写，新加的主题手机上自动有一套。
//! 键面取 `surface`，按下取 `accent`，文字取 `text` / `gloss` / `faint` / `fresh` / `cloud`；底板、功能键、回车键按下面的规则从它们推。
//! 推导规则见 `docs/design/theme.md`「手机键盘」。

use bamboo_slip::Color;

use super::Theme;

/// 回车键（品牌色）与白字至少要有的对比度（WCAG 相对亮度之比）；不够就把品牌色调暗。
const BRAND_CONTRAST: f32 = 3.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyboardColors {
    /// 键面是深色（按 `surface` 的亮度判断，不看外观设置：锁定外观的主题照样对）。
    pub dark: bool,

    /// 键盘底板。
    pub board: Color,

    /// 字母键与候选条的面（主题的 `surface`）。
    pub key: Color,

    /// 功能键（删除、符号、123 这些）。
    pub function_key: Color,

    /// 按下的键与首选高亮（主题的 `accent` 叠在键面上）。
    pub pressed: Color,

    /// 回车键与品牌色。
    pub brand: Color,

    /// 回车键上的字。
    pub on_brand: Color,

    /// 键下的投影。
    pub shadow: Color,

    pub text: Color,

    pub gloss: Color,

    pub faint: Color,

    /// 生词的译文。
    pub fresh: Color,

    /// 云联想的词。
    pub cloud: Color,
}

impl KeyboardColors {
    /// 按主题当前的外观推出一套。主题缺的变量用青简绿的值（浅色 / 深色按键面亮度）。
    pub fn of(theme: &Theme) -> Self {
        let get = |name: &str, light: Color, dark: Color| {
            theme
                .variable(name)
                .unwrap_or(if theme.dark { dark } else { light })
        };
        let backdrop = if theme.dark {
            Color::rgb(0, 0, 0)
        } else {
            Color::rgb(255, 255, 255)
        };
        let surface = over(
            get(
                "surface",
                Color::rgb(255, 255, 255),
                Color::rgb(0x1e, 0x1e, 0x1e),
            ),
            backdrop,
        );
        let dark = luminance(surface) < 0.4;
        let accent = get(
            "accent",
            Color::rgba(0x94, 0xbe, 0x52, 0x7f),
            Color::rgb(0x33, 0x6f, 0x33),
        );
        // 底板与功能键带一点高亮色的色相，键盘才像这个主题的
        let tint = opaque(accent);
        let black = Color::rgb(0, 0, 0);
        let white = Color::rgb(255, 255, 255);
        let (key, board, function_key, shadow) = if dark {
            let key = mix(surface, white, 0.08);
            (
                key,
                mix(mix(surface, tint, 0.04), black, 0.35),
                mix(mix(surface, tint, 0.06), white, 0.02),
                Color::rgba(0, 0, 0, 0x66),
            )
        } else {
            (
                surface,
                mix(mix(surface, tint, 0.08), black, 0.06),
                mix(mix(surface, tint, 0.12), black, 0.13),
                Color::rgba(0, 0, 0, 0x2e),
            )
        };
        Self {
            dark,
            board,
            key,
            function_key,
            pressed: over(accent, key),
            brand: readable_on_white(tint),
            on_brand: white,
            shadow,
            text: get(
                "text",
                Color::rgba(0, 0, 0, 0xd8),
                Color::rgba(255, 255, 255, 0xd8),
            ),
            gloss: get(
                "gloss",
                Color::rgba(0, 0, 0, 0xa6),
                Color::rgba(255, 255, 255, 0xb3),
            ),
            faint: get(
                "faint",
                Color::rgba(0, 0, 0, 0x73),
                Color::rgba(255, 255, 255, 0x73),
            ),
            fresh: get(
                "fresh",
                Color::rgb(0xff, 0x8d, 0x28),
                Color::rgb(0xff, 0x92, 0x30),
            ),
            cloud: get(
                "cloud",
                Color::rgb(0x00, 0xc3, 0xd0),
                Color::rgb(0x00, 0xd2, 0xe0),
            ),
        }
    }
}

/// `top` 按它的不透明度叠在不透明的 `bottom` 上。
fn over(top: Color, bottom: Color) -> Color {
    let mut color = mix(bottom, top, f32::from(top.a) / 255.0);
    color.a = 255;
    color
}

/// 去掉不透明度，只留色相。
fn opaque(color: Color) -> Color {
    Color { a: 255, ..color }
}

/// 按比例 `t`（0 全是 `a`、1 全是 `b`）混两种颜色。
fn mix(a: Color, b: Color, t: f32) -> Color {
    let channel = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
    Color::rgba(
        channel(a.r, b.r),
        channel(a.g, b.g),
        channel(a.b, b.b),
        channel(a.a, b.a),
    )
}

/// WCAG 相对亮度，0 黑 1 白。
fn luminance(color: Color) -> f32 {
    let linear = |c: u8| {
        let c = f32::from(c) / 255.0;
        if c <= 0.039_28 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
}

/// 白字在这个颜色上的对比度。
fn contrast_with_white(color: Color) -> f32 {
    1.05 / (luminance(color) + 0.05)
}

/// 白字放得上去的品牌色：保持色相与饱和度，按 HSL 亮度一档档调暗到对比度够为止。
fn readable_on_white(color: Color) -> Color {
    let (h, s, mut l) = to_hsl(color);
    let mut result = color;
    while contrast_with_white(result) < BRAND_CONTRAST && l > 0.0 {
        l = (l - 0.01).max(0.0);
        result = from_hsl(h, s, l);
    }
    result
}

fn to_hsl(color: Color) -> (f32, f32, f32) {
    let [r, g, b] = [color.r, color.g, color.b].map(|c| f32::from(c) / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;
    if d == 0.0 {
        return (0.0, 0.0, l);
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let h = if max == r {
        ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h * 60.0, s, l)
}

fn from_hsl(h: f32, s: f32, l: f32) -> Color {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let channel = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    Color::rgb(channel(r), channel(g), channel(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_builtin_theme_gets_a_usable_keyboard() {
        for theme in Theme::builtins() {
            for dark in [false, true] {
                let theme = theme.with_dark(dark);
                let colors = KeyboardColors::of(&theme);
                let what = format!("{} dark={dark}", theme.id());
                assert!(
                    contrast_with_white(colors.brand) >= BRAND_CONTRAST,
                    "{what}：回车键上的白字看不清"
                );
                assert_ne!(colors.board, colors.key, "{what}：键与底板分不开");
                assert_ne!(colors.pressed, colors.key, "{what}：按下看不出来");
                assert_eq!(colors.key.a, 255, "{what}：键面要不透明");
                // 锁定浅色的主题深色时仍是浅色键盘
                if theme.locked_dark() == Some(false) {
                    assert!(!colors.dark, "{what}");
                }
            }
        }
    }

    #[test]
    fn qingjian_keeps_its_look() {
        let light = KeyboardColors::of(&Theme::light());
        assert!(!light.dark);
        assert_eq!(light.key, Color::rgb(255, 255, 255));
        // 半透明的高亮叠在白键上
        assert_eq!(light.pressed, Color::rgb(0xca, 0xdf, 0xa9));
        let dark = KeyboardColors::of(&Theme::dark());
        assert!(dark.dark);
        assert!(luminance(dark.board) < luminance(dark.key));
    }

    #[test]
    fn hsl_round_trips() {
        for color in [
            Color::rgb(0x94, 0xbe, 0x52),
            Color::rgb(0x00, 0x7a, 0xff),
            Color::rgb(0xf8, 0xcb, 0xe9),
        ] {
            let (h, s, l) = to_hsl(color);
            assert_eq!(from_hsl(h, s, l), color);
        }
    }
}
