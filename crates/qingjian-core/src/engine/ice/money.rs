//! 数字和金额的四种写法；金额保留角、分、厘、毫。

use super::tools::candidate;
use crate::candidate::Candidate;
use crate::shortcut::{chinese_decimal_lower, chinese_decimal_upper, chinese_lower, chinese_upper};

pub(super) fn forms(input: &str) -> Vec<Candidate> {
    let (integer, fraction) = input.split_once('.').unwrap_or((input, ""));
    if integer.is_empty()
        || integer.len() > 12
        || !integer.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
    {
        return Vec::new();
    }
    let decimal = input.contains('.');
    let lower = if decimal {
        chinese_decimal_lower(integer, fraction)
    } else {
        chinese_lower(integer)
    }
    .replace('零', "〇");
    let upper = if decimal {
        chinese_decimal_upper(integer, fraction)
    } else {
        chinese_upper(integer)
    }
    .replace('万', "萬")
    .replace('亿', "億");
    let amount = |upper: bool| {
        let digits = if upper {
            ["零", "壹", "贰", "叁", "肆", "伍", "陆", "柒", "捌", "玖"]
        } else {
            ["〇", "一", "二", "三", "四", "五", "六", "七", "八", "九"]
        };
        let mut value = if upper {
            chinese_upper(integer)
        } else {
            chinese_lower(integer).replace('零', "〇")
        };
        value.push('元');
        let fraction = &fraction[..fraction.len().min(4)];
        let fraction = fraction.trim_end_matches('0');
        if fraction.is_empty() {
            value.push('整');
        } else {
            let mut zero = false;
            for (digit, unit) in fraction.bytes().zip(["角", "分", "厘", "毫"]) {
                if digit == b'0' {
                    if !zero {
                        value.push_str(digits[0]);
                    }
                    zero = true;
                } else {
                    value.push_str(digits[usize::from(digit - b'0')]);
                    value.push_str(unit);
                    zero = false;
                }
            }
        }
        value
    };
    [
        (lower, "〔数字小写〕"),
        (upper, "〔数字大写〕"),
        (amount(false), "〔金额小写〕"),
        (amount(true), "〔金额大写〕"),
    ]
    .into_iter()
    .map(|(text, comment)| candidate(text, Some(comment.to_owned())))
    .collect()
}
