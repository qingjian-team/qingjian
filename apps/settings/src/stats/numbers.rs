//! 给人看的数：千位分隔、万 / 亿字、Token 数、跟一本书比。

use qingjian_core::book_scale;

use crate::i18n::{tr, tr_args};

/// 千位分隔：`12345` → `12,345`（分隔符随界面语言）。
pub fn group_digits(value: u64) -> String {
    let separator = tr("numbers.thousands_separator");
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 * separator.len());
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push_str(separator);
        }
        out.push(digit);
    }
    out
}

/// 字数：万以下写整数（`2,500 字`），万以上一位小数（`12.3 万字`，英文 `123K characters`）。
pub fn hanzi_text(value: u64) -> String {
    match compact(value) {
        Some((number, unit)) => tr_args("numbers.hanzi_large", &[("n", &number), ("unit", &unit)]),
        None => tr_args("numbers.hanzi", &[("n", &group_digits(value))]),
    }
}

/// 「累计 12.3 万字，约等于 1.7 本《活着》（约 12 万字）」；一个字没有时不比。
pub fn scale_line(hanzi: u64) -> String {
    if hanzi == 0 {
        return tr("numbers.no_input_yet").to_owned();
    }
    let (book, ratio) = book_scale(hanzi);
    tr_args(
        "numbers.scale",
        &[
            ("total", &hanzi_text(hanzi)),
            ("ratio", &format_ratio(ratio)),
            ("book", &book.title),
            ("book_size", &hanzi_text(book.hanzi)),
        ],
    )
}

/// Token 数：万以下千位分隔，万以上一位小数（`12.3 万`、`123K`）。
pub fn tokens_text(value: u64) -> String {
    match compact(value) {
        Some((number, unit)) => tr_args("numbers.compact", &[("n", &number), ("unit", &unit)]),
        None => group_digits(value),
    }
}

/// 一万以上的数拆成一位小数与单位：界面语言按万进（中文、日文）就是万 / 亿，否则千 / 百万 / 十亿。
pub fn compact(value: u64) -> Option<(String, &'static str)> {
    if value < 10_000 {
        return None;
    }
    let units: &[(f64, &str)] = if tr("numbers.myriad") == "true" {
        &[(1e8, "numbers.unit_2"), (1e4, "numbers.unit_1")]
    } else {
        &[
            (1e9, "numbers.unit_3"),
            (1e6, "numbers.unit_2"),
            (1e3, "numbers.unit_1"),
        ]
    };
    let value = value as f64;
    units
        .iter()
        .find(|(size, _)| value >= *size)
        .map(|(size, unit)| (trim_decimal(value / size), tr(unit)))
}

/// 一位小数，`.0` 去掉；小数点随界面语言。
fn trim_decimal(value: f64) -> String {
    let text = format!("{value:.1}");
    let text = text.strip_suffix(".0").map_or(text.clone(), str::to_owned);
    text.replace('.', tr("numbers.decimal_separator"))
}

/// 倍数：不到 10 倍留一位小数，再多就取整。
fn format_ratio(ratio: f64) -> String {
    if ratio >= 10.0 {
        format!("{}", ratio.round() as u64)
    } else {
        trim_decimal(ratio)
    }
}

#[cfg(test)]
mod tests {
    use super::{group_digits, hanzi_text, scale_line, tokens_text};

    #[test]
    fn numbers_read_in_chinese_units() {
        bamboo_slip_i18n::pin_locale("zh-Hans");
        assert_eq!(group_digits(1_234_567), "1,234,567");
        assert_eq!(hanzi_text(9_999), "9,999 字");
        assert_eq!(hanzi_text(123_000), "12.3 万字");
        assert_eq!(hanzi_text(250_000_000), "2.5 亿字");
        assert!(scale_line(204_000).starts_with("累计 20.4 万字，约等于"));
        assert_eq!(tokens_text(9_999), "9,999");
        assert_eq!(tokens_text(1_230_000), "123 万");
    }
}
