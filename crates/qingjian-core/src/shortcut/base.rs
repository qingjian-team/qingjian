//! `base` 子模式：进制转换，`vbase;128;16` → `80`。
//!
//! 值可以带前缀（`0x` / `0b` / `0o`），不带前缀按十进制读；目标进制 2–36。
//! 参数之间用分号隔开（全角半角都认），与其它带参数的子模式一致。

use super::modes::split_args;

/// 各进制用的数字/字母，最多 36 进制。
const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";

/// `base;<值>;<目标进制>`。
pub fn forms(args: &str) -> Vec<String> {
    let parts = split_args(args);
    let [value, target] = parts[..] else {
        return Vec::new();
    };
    let Some((number, from)) = parse_value(value) else {
        return Vec::new();
    };
    let Ok(target) = target.parse::<u32>() else {
        return Vec::new();
    };
    if !(2..=36).contains(&target) {
        return Vec::new();
    }

    let rendered = render(number, target);
    let mut forms = vec![
        rendered.clone(),
        format!("{value}({from}) = {rendered}({target})"),
    ];
    // 该进制有常用前缀写法时再给一条，方便直接粘进代码
    match target {
        2 => forms.push(format!("0b{rendered}")),
        8 => forms.push(format!("0o{rendered}")),
        16 => forms.push(format!("0x{rendered}")),
        _ => {}
    }
    forms
}

/// 把值读成整数与它的源进制。`0x` / `0b` / `0o` 前缀决定源进制，否则按十进制。
fn parse_value(text: &str) -> Option<(i128, u32)> {
    let lower = text.trim().to_ascii_lowercase();
    let (negative, body) = match lower.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, lower.as_str()),
    };
    let (radix, digits) = if let Some(rest) = body.strip_prefix("0x") {
        (16, rest)
    } else if let Some(rest) = body.strip_prefix("0b") {
        (2, rest)
    } else if let Some(rest) = body.strip_prefix("0o") {
        (8, rest)
    } else {
        (10, body)
    };
    let magnitude = i128::from_str_radix(digits, radix).ok()?;
    Some((if negative { -magnitude } else { magnitude }, radix))
}

/// 整数按某个进制写出来，负数带负号。
fn render(value: i128, radix: u32) -> String {
    let magnitude = value.unsigned_abs();
    let body = to_radix(magnitude, radix);
    if value < 0 { format!("-{body}") } else { body }
}

/// 无符号整数转任意进制（2–36）。
fn to_radix(mut value: u128, radix: u32) -> String {
    if value == 0 {
        return "0".to_owned();
    }
    let radix = u128::from(radix);
    let mut digits = Vec::new();
    while value > 0 {
        digits.push(DIGITS[(value % radix) as usize]);
        value /= radix;
    }
    digits.reverse();
    String::from_utf8(digits).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_between_radixes() {
        assert_eq!(forms("128;16")[0], "80");
        assert_eq!(forms("255;2")[0], "11111111");
        assert_eq!(forms("255;8")[0], "377");
        assert_eq!(forms("0xff;10")[0], "255");
        assert_eq!(forms("0b1010;16")[0], "a");
        assert_eq!(forms("0o777;10")[0], "511");
    }

    #[test]
    fn keeps_prefix_and_negative() {
        let hex = forms("128;16");
        assert!(hex.iter().any(|f| f == "0x80"), "{hex:?}");
        let bin = forms("5;2");
        assert!(bin.iter().any(|f| f == "0b101"), "{bin:?}");
        // 负数保留符号
        assert_eq!(forms("-128;16")[0], "-80");
        // 标注式说明源与目标
        assert!(forms("128;16").iter().any(|f| f == "128(10) = 80(16)"));
    }

    #[test]
    fn rejects_bad_arguments() {
        assert!(forms("128").is_empty());
        assert!(forms("128;37").is_empty());
        assert!(forms("128;1").is_empty());
        assert!(forms("abc;16").is_empty());
        assert!(forms("").is_empty());
    }

    #[test]
    fn accepts_full_width_separator() {
        assert_eq!(forms("128；16")[0], "80");
    }
}
