//! 快捷候选（搜狗「v 模式」的那一套）：不查词库、由输入本身直接算出来的候选。
//!
//! - `rq` / `sj` / `xq`：今天的日期、现在的时间、星期几，插在本地候选第二位起。
//! - 表达式键（缺省 `v`）开头进表达式模式：`v1+2` 出 `3` 与 `1+2=3`，`v123` 出中文数字（小写与大写）。
//!   表达式模式下缓冲区允许数字与运算符，候选不走拼音解析。
//! - 问字键（缺省 `u`）后跟十六进制码点出那个字符：`u4e00` → 一，`u+1f600` → 😀。模式键本身在 `engine::ModeKeys`。

mod calendar;
mod evaluator;
mod numeral;

use jiff::Zoned;

use crate::candidate::{Candidate, CandidateKind};

pub use calendar::{date_forms, time_forms, weekday_forms};
pub use evaluator::evaluate;
pub use numeral::{chinese_lower, chinese_upper};

/// 表达式模式的缺省前缀。`v` 不是任何拼音音节的开头，用它不会和拼音冲突。
pub const EXPRESSION_PREFIX: char = 'v';

/// 码点最长几位十六进制（U+10FFFF）。
const MAX_CODEPOINT_DIGITS: usize = 6;

/// 问字前缀之后的部分是不是 Unicode 码点：`4e00`、`+1f600`。全是十六进制且至少含一个数字，
/// 或以 `+` 开头（`+face` 这种纯字母也算）；纯字母不带 `+` 当拼音问题（`fade` 是合法拼音）。
/// 是码点就返回那个字符（不可显示的控制字符、代理区、超范围返回 `None`）。
pub fn unicode_form(body: &str) -> Option<String> {
    let (explicit, hex) = match body.strip_prefix('+') {
        Some(rest) => (true, rest),
        None => (false, body),
    };
    if hex.is_empty()
        || hex.len() > MAX_CODEPOINT_DIGITS
        || !hex.bytes().all(|b| b.is_ascii_hexdigit())
        || (!explicit && !hex.bytes().any(|b| b.is_ascii_digit()))
    {
        return None;
    }
    let value = u32::from_str_radix(hex, 16).ok()?;
    let c = char::from_u32(value)?;
    (!c.is_control()).then(|| c.to_string())
}

/// 到目前为止敲的还可能是码点（空、`+`、或全是十六进制）：壳据此决定数字进缓冲区还是选词。
pub fn could_be_unicode(body: &str) -> bool {
    let hex = body.strip_prefix('+').unwrap_or(body);
    hex.len() <= MAX_CODEPOINT_DIGITS && hex.bytes().all(|b| b.is_ascii_hexdigit())
}

/// 表达式模式下允许敲进缓冲区的非字母字符：数字与四则运算符号。
/// 字母（`x` 当乘号）本来就能进缓冲区，不在此列。
pub fn is_expression_char(c: char) -> bool {
    c.is_ascii_digit() || matches!(c, '+' | '-' | '*' | '/' | '(' | ')' | '.' | '^')
}

/// 按输入算快捷候选；不是快捷输入时为空。`expression` 是表达式键；`now` 由调用方给，测试可固定时间。
pub fn candidates(input: &str, expression: char, now: &Zoned) -> Vec<Candidate> {
    let texts: Vec<String> = match input {
        "rq" => date_forms(now),
        "sj" => time_forms(now),
        "xq" => weekday_forms(now),
        _ => match input.strip_prefix(expression) {
            Some(body) => expression_forms(body),
            None => Vec::new(),
        },
    };
    texts
        .into_iter()
        .map(|text| Candidate {
            text,
            kind: CandidateKind::Shortcut,
            syllables: Vec::new(),
            reading: None,
            translation: None,
        })
        .collect()
}

/// 表达式键之后的部分：纯数字出中文数字，四则运算出结果与「算式=结果」。
fn expression_forms(body: &str) -> Vec<String> {
    if body.is_empty() {
        return Vec::new();
    }
    if body.bytes().all(|b| b.is_ascii_digit()) {
        return vec![chinese_lower(body), chinese_upper(body)];
    }
    match evaluate(body) {
        Some(result) => vec![result.clone(), format!("{body}={result}")],
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;
    use jiff::tz::TimeZone;

    use super::*;

    #[test]
    fn unicode_bodies_need_a_digit_or_a_plus() {
        assert_eq!(unicode_form("4e00").as_deref(), Some("一"));
        assert_eq!(unicode_form("+1f600").as_deref(), Some("😀"));
        assert_eq!(unicode_form("+face").as_deref(), Some("\u{face}"));
        // 纯字母没有 + 当拼音问题（fade 是合法拼音）
        assert_eq!(unicode_form("fade"), None);
        assert_eq!(unicode_form(""), None);
        assert_eq!(unicode_form("+"), None);
        // 控制字符、代理区、超范围
        assert_eq!(unicode_form("07"), None);
        assert_eq!(unicode_form("d800"), None);
        assert_eq!(unicode_form("110000"), None);
        assert_eq!(unicode_form("1234567"), None);
        assert!(could_be_unicode(""));
        assert!(could_be_unicode("+"));
        assert!(could_be_unicode("4e"));
        assert!(could_be_unicode("face"));
        assert!(!could_be_unicode("sangemu"));
    }

    fn now() -> Zoned {
        date(2026, 9, 3)
            .at(19, 6, 23, 0)
            .to_zoned(TimeZone::UTC)
            .unwrap()
    }

    fn texts(input: &str) -> Vec<String> {
        candidates(input, EXPRESSION_PREFIX, &now())
            .into_iter()
            .map(|c| c.text)
            .collect()
    }

    #[test]
    fn calendar_shortcuts() {
        assert_eq!(texts("rq"), ["2026年9月3日", "2026-09-03", "2026/09/03"]);
        assert_eq!(texts("sj"), ["19:06", "19:06:23", "19点06分"]);
        assert_eq!(texts("xq"), ["星期四", "周四"]);
        assert!(texts("rqi").is_empty());
    }

    #[test]
    fn expression_shortcuts() {
        assert_eq!(texts("v1+2"), ["3", "1+2=3"]);
        assert_eq!(texts("v123"), ["一百二十三", "壹佰贰拾叁"]);
        assert!(texts("v").is_empty());
        assert!(texts("v1+").is_empty());
        assert!(texts("very").is_empty());
    }

    #[test]
    fn expression_chars() {
        assert!(is_expression_char('7'));
        assert!(is_expression_char('('));
        assert!(!is_expression_char('='));
        assert!(!is_expression_char('x'));
    }
}
