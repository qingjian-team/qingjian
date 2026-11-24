//! `col` 子模式：认一个颜色，给出各种写法，`vcol;#ff8800`、`vcol;255;136;0`、`vcol;hsl(32,100%,50%)`。
//!
//! 解析交给 `csscolorparser`（hex、`rgb()` / `rgba()` / `hsl()` / `hsla()`、颜色名），
//! 换算交给 `palette`（HSL、HSV、CMYK、Lab）。这里只做三件事：
//! 把参数凑成一个颜色、把各写法拼成候选、以及色温与颜色互转（黑体近似，没有现成库）。

use csscolorparser::Color;
use palette::{FromColor, Hsl, Hsv, IntoColor, Lab, RgbHue, Srgb, Xyz};

/// 一条候选：上屏的值，与只在界面上显示的名字（空串表示不带名字）。
pub type Measure = (String, String);

/// 认出来的一个颜色：sRGB 三个分量（0–1）与不透明度（0–1）。
#[derive(Debug, Clone, Copy, PartialEq)]
struct Rgba {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

/// `参数` → 各种颜色写法。认不出来时为空。
pub fn forms(args: &str) -> Vec<Measure> {
    let Some(color) = parse(args) else {
        return Vec::new();
    };
    let srgb: Srgb<f32> = Srgb::new(color.r, color.g, color.b);
    let (r8, g8, b8) = (byte(color.r), byte(color.g), byte(color.b));
    let a8 = byte(color.a);
    let opaque = a8 == 255;

    let hsl: Hsl = srgb.into_color();
    let hsv: Hsv = srgb.into_color();
    let (c, m, y, k) = cmyk(color);
    let lab: Lab = srgb.into_color();

    let mut forms = vec![
        (format!("#{r8:02x}{g8:02x}{b8:02x}"), "HEX".to_owned()),
        (format!("rgb({r8}, {g8}, {b8})"), "RGB".to_owned()),
        (
            format!(
                "hsl({}, {}%, {}%)",
                degrees(hsl.hue),
                percent(hsl.saturation),
                percent(hsl.lightness)
            ),
            "HSL".to_owned(),
        ),
        (
            format!(
                "hsv({}, {}%, {}%)",
                degrees(hsv.hue),
                percent(hsv.saturation),
                percent(hsv.value)
            ),
            "HSV / HSB".to_owned(),
        ),
        (
            format!(
                "cmyk({}%, {}%, {}%, {}%)",
                percent(c),
                percent(m),
                percent(y),
                percent(k)
            ),
            "CMYK".to_owned(),
        ),
        (
            format!("lab({:.1}, {:.1}, {:.1})", lab.l, lab.a, lab.b),
            "Lab".to_owned(),
        ),
        (
            format!("{:.3} {:.3} {:.3}", color.r, color.g, color.b),
            "浮点 RGB（0–1）".to_owned(),
        ),
        (format!("{r8} {g8} {b8}"), "整数 RGB（0–255）".to_owned()),
    ];
    // 带透明度的写法只在真带透明度时给：不透明的颜色给一堆 ff 没意义
    if !opaque {
        forms.push((
            format!("#{r8:02x}{g8:02x}{b8:02x}{a8:02x}"),
            "HEX（带透明度）".to_owned(),
        ));
        forms.push((
            format!("rgba({r8}, {g8}, {b8}, {})", float_alpha(color.a)),
            "RGBA".to_owned(),
        ));
        forms.push((
            format!(
                "{:.3} {:.3} {:.3} {}",
                color.r,
                color.g,
                color.b,
                float_alpha(color.a)
            ),
            "浮点 RGBA".to_owned(),
        ));
    }
    // 色温：由色度反推相关色温（McCamy 近似），灰阶没有意义就不给
    if let Some(kelvin) = correlated_temperature(srgb) {
        forms.push((format!("{kelvin}K"), "色温".to_owned()));
    }
    forms
}

/// 参数 → 颜色。认这几种写法：
///
/// - 带方案名：`rgb;255;136;0`、`hsl;32;100;50`、`hsv;32;100;100`、`cmyk;0;47;100;0`、
///   `lab;67;40;70`、`hex;ff8800`、`kelvin;2700`（`hsb` 是 `hsv` 的别名）
/// - CSS：`#ff8800`、`#f80`、`#ff880080`、`rgb(255,136,0)`、`hsl(32,100%,50%)`、颜色名（`tomato`）
/// - 光写数：3 个是 RGB（0–255，或都 ≤1 时按 0–1）、第 4 个是透明度
/// - 色温：`5000k`
///
/// 百分比与 0–1 两种都认：`hsl;32;1;0.5` 与 `hsl;32;100;50` 是同一个颜色。
fn parse(args: &str) -> Option<Rgba> {
    let text = args.trim();
    if text.is_empty() {
        return None;
    }
    let parts: Vec<&str> = text
        .split([';', '；', '：', ':'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    // 带方案名的那种：第一个词是方案，其余是数
    if parts.len() >= 2
        && let Some(color) = by_scheme(parts[0], &parts[1..])
    {
        return Some(color);
    }
    if parts.len() >= 2 && is_scheme(parts[0]) {
        // 方案名认出来了，但参数不对：不给候选，别猜
        return None;
    }
    // 色温：`5000k`
    let lowered = text.to_ascii_lowercase();
    if let Some(rest) = lowered.strip_suffix('k')
        && let Ok(kelvin) = rest.trim().parse::<f32>()
    {
        return kelvin_rgba(kelvin);
    }
    // CSS 那套（hex、函数式、颜色名）：带字母或 `#` 的都先交给它
    if text.starts_with('#') || text.chars().any(|c| c.is_ascii_alphabetic()) {
        return csscolorparser::parse(text).ok().map(from_css);
    }
    // 光写数
    let numbers = numbers(text)?;
    match numbers.len() {
        3 => Some(Rgba {
            r: unit(numbers[0]),
            g: unit(numbers[1]),
            b: unit(numbers[2]),
            a: 1.0,
        }),
        4 => Some(Rgba {
            r: unit(numbers[0]),
            g: unit(numbers[1]),
            b: unit(numbers[2]),
            a: alpha_unit(numbers[3]),
        }),
        _ => None,
    }
}

/// 方案名认不认。
fn is_scheme(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "rgb"
            | "rgba"
            | "hsl"
            | "hsla"
            | "hsv"
            | "hsb"
            | "hsva"
            | "cmyk"
            | "lab"
            | "hex"
            | "kelvin"
            | "k"
    )
}

/// 按方案名算颜色。
fn by_scheme(name: &str, rest: &[&str]) -> Option<Rgba> {
    let name = name.to_ascii_lowercase();
    let mut values = Vec::with_capacity(rest.len());
    for part in rest {
        values.push(part.parse::<f32>().ok()?);
    }
    let alpha = |values: &[f32]| values.get(3).map_or(1.0, |value| alpha_unit(*value));
    match name.as_str() {
        "rgb" | "rgba" if values.len() == 3 || values.len() == 4 => Some(Rgba {
            r: unit(values[0]),
            g: unit(values[1]),
            b: unit(values[2]),
            a: alpha(&values),
        }),
        "hsl" | "hsla" if values.len() == 3 || values.len() == 4 => {
            let hsl = Hsl::new_srgb(values[0], ratio(values[1]), ratio(values[2]));
            let srgb: Srgb<f32> = Srgb::from_color(hsl);
            Some(Rgba {
                r: srgb.red,
                g: srgb.green,
                b: srgb.blue,
                a: alpha(&values),
            })
        }
        "hsv" | "hsb" | "hsva" if values.len() == 3 || values.len() == 4 => {
            let hsv = Hsv::new_srgb(values[0], ratio(values[1]), ratio(values[2]));
            let srgb: Srgb<f32> = Srgb::from_color(hsv);
            Some(Rgba {
                r: srgb.red,
                g: srgb.green,
                b: srgb.blue,
                a: alpha(&values),
            })
        }
        "cmyk" if values.len() == 4 || values.len() == 5 => {
            let (c, m, y, k) = (
                ratio(values[0]),
                ratio(values[1]),
                ratio(values[2]),
                ratio(values[3]),
            );
            let scale = 1.0 - k;
            Some(Rgba {
                r: (1.0 - c) * scale,
                g: (1.0 - m) * scale,
                b: (1.0 - y) * scale,
                a: values.get(4).map_or(1.0, |value| alpha_unit(*value)),
            })
        }
        "lab" if values.len() == 3 || values.len() == 4 => {
            let srgb: Srgb<f32> = Srgb::from_color(Lab::new(values[0], values[1], values[2]));
            Some(Rgba {
                r: srgb.red.clamp(0.0, 1.0),
                g: srgb.green.clamp(0.0, 1.0),
                b: srgb.blue.clamp(0.0, 1.0),
                a: alpha(&values),
            })
        }
        "hex" if rest.len() == 1 => csscolorparser::parse(rest[0]).ok().map(from_css),
        "kelvin" | "k" if values.len() == 1 => kelvin_rgba(values[0]),
        _ => None,
    }
}

/// 百分比与 0–1 都认：大于 1 的按百分数折回。
fn ratio(value: f32) -> f32 {
    if value > 1.0 {
        (value / 100.0).clamp(0.0, 1.0)
    } else {
        value.clamp(0.0, 1.0)
    }
}

fn from_css(color: Color) -> Rgba {
    Rgba {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    }
}

/// 分号（全角也认）分开的数。
fn numbers(text: &str) -> Option<Vec<f32>> {
    let mut values = Vec::new();
    for part in text.split([';', '；']) {
        if part.trim().is_empty() {
            continue;
        }
        values.push(part.trim().parse().ok()?);
    }
    Some(values)
}

/// 分量：0–1 的原样，0–255 的折回 0–1。
fn unit(value: f32) -> f32 {
    if value > 1.0 {
        (value / 255.0).clamp(0.0, 1.0)
    } else {
        value.clamp(0.0, 1.0)
    }
}

/// 透明度：0–1 的原样，大于 1 的按 0–255 折回。
fn alpha_unit(value: f32) -> f32 {
    if value > 1.0 {
        (value / 255.0).clamp(0.0, 1.0)
    } else {
        value.clamp(0.0, 1.0)
    }
}

fn byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// sRGB → CMYK（`palette` 不含 CMYK：它不是色度空间，是印刷用的换算）。
fn cmyk(color: Rgba) -> (f32, f32, f32, f32) {
    let k = 1.0 - color.r.max(color.g).max(color.b);
    if k >= 1.0 {
        return (0.0, 0.0, 0.0, 1.0);
    }
    let inverse = 1.0 - k;
    (
        (1.0 - color.r - k) / inverse,
        (1.0 - color.g - k) / inverse,
        (1.0 - color.b - k) / inverse,
        k,
    )
}

fn degrees(hue: RgbHue) -> f32 {
    hue.into_positive_degrees().round()
}

fn percent(value: f32) -> f32 {
    (value * 100.0).clamp(0.0, 100.0).round()
}

/// 透明度写成 0–1 的短串（最多三位小数，不留尾随零）。
fn float_alpha(value: f32) -> String {
    let text = format!("{:.3}", value.clamp(0.0, 1.0));
    let trimmed = text.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() {
        "0".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// 色度反推相关色温（McCamy 近似）；灰阶与太偏的色度返回 `None`。
fn correlated_temperature(srgb: Srgb<f32>) -> Option<u32> {
    let xyz: Xyz = srgb.into_color();
    let sum = xyz.x + xyz.y + xyz.z;
    if sum <= 0.0 {
        return None;
    }
    let (x, y) = (xyz.x / sum, xyz.y / sum);
    let n = (x - 0.3320) / (0.1858 - y);
    let kelvin = 449.0 * n.powi(3) + 3525.0 * n.powi(2) + 6823.3 * n + 5520.33;
    (1000.0..=40000.0)
        .contains(&kelvin)
        .then(|| kelvin.round() as u32)
}

/// 色温 → sRGB（黑体近似，Tanner Helland 那一套；只在 1000–40000 K 有意义）。
///
/// 这颗没有现成的库：`palette` 只管色彩空间换算，不含黑体光谱。二十来行，够用。
fn kelvin_rgba(kelvin: f32) -> Option<Rgba> {
    if !(1000.0..=40000.0).contains(&kelvin) {
        return None;
    }
    let temperature = kelvin / 100.0;
    let red = if temperature <= 66.0 {
        255.0
    } else {
        329.698_73 * (temperature - 60.0).powf(-0.133_204_76)
    };
    let green = if temperature <= 66.0 {
        99.470_8 * temperature.ln() - 161.119_57
    } else {
        288.122_16 * (temperature - 60.0).powf(-0.075_514_846)
    };
    let blue = if temperature >= 66.0 {
        255.0
    } else if temperature <= 19.0 {
        0.0
    } else {
        138.517_73 * (temperature - 10.0).ln() - 305.044_8
    };
    Some(Rgba {
        r: (red.clamp(0.0, 255.0) / 255.0),
        g: (green.clamp(0.0, 255.0) / 255.0),
        b: (blue.clamp(0.0, 255.0) / 255.0),
        a: 1.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(args: &str, label: &str) -> Option<String> {
        forms(args)
            .into_iter()
            .find(|(_, name)| name == label)
            .map(|(value, _)| value)
    }

    #[test]
    fn hex_and_named_and_functional_agree() {
        assert_eq!(value("#ff8800", "RGB").as_deref(), Some("rgb(255, 136, 0)"));
        assert_eq!(value("#f80", "HEX").as_deref(), Some("#ff8800"));
        assert_eq!(value("rgb(255,136,0)", "HEX").as_deref(), Some("#ff8800"));
        assert_eq!(value("tomato", "HEX").as_deref(), Some("#ff6347"));
        assert_eq!(
            value("hsl(32,100%,50%)", "HEX").as_deref(),
            Some("#ff8800"),
            "HSL 与 HEX 应当回到同一个颜色"
        );
    }

    #[test]
    fn bare_numbers_are_rgb_and_accept_floats() {
        assert_eq!(value("255;136;0", "HEX").as_deref(), Some("#ff8800"));
        // 全 ≤1 的按 0–1
        assert_eq!(value("1;0.533;0", "HEX").as_deref(), Some("#ff8800"));
        assert_eq!(
            value("1;0.533;0", "浮点 RGB（0–1）").as_deref(),
            Some("1.000 0.533 0.000")
        );
    }

    #[test]
    fn alpha_only_shows_when_present() {
        assert!(value("#ff8800", "HEX（带透明度）").is_none());
        assert_eq!(
            value("#ff880080", "HEX（带透明度）").as_deref(),
            Some("#ff880080")
        );
        assert_eq!(
            value("255;136;0;128", "RGBA").as_deref(),
            Some("rgba(255, 136, 0, 0.502)")
        );
    }

    #[test]
    fn other_spaces_come_out_too() {
        assert_eq!(
            value("#ff8800", "HSL").as_deref(),
            Some("hsl(32, 100%, 50%)")
        );
        assert!(value("#ff8800", "HSV / HSB").is_some());
        assert!(value("#ff8800", "CMYK").is_some());
        assert!(value("#ff8800", "Lab").is_some());
        assert!(value("#ff8800", "整数 RGB（0–255）").is_some());
    }

    #[test]
    fn kelvin_in_and_out() {
        // 6500K 接近白，色温能反推回来
        let warm = value("kelvin;2700", "HEX");
        assert!(
            warm.as_deref().is_some_and(|hex| hex.starts_with("#ff")),
            "{warm:?}"
        );
        let back = value("kelvin;2700", "色温");
        assert!(back.is_some(), "由颜色反推色温");
        assert!(value("5000k", "HEX").is_some(), "`5000k` 也认");
        assert!(forms("kelvin;500").is_empty(), "低于 1000K 不认");
    }

    #[test]
    fn garbage_gives_nothing() {
        assert!(forms("").is_empty());
        assert!(forms("不是颜色").is_empty());
        assert!(forms("1;2").is_empty(), "两个数不成颜色");
        assert!(forms("rgb(300,0,0").is_empty(), "括号没闭合");
    }
}
