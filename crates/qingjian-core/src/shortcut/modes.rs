//! `v` 模式下的子模式路由：`v<子模式>:<语句>`。
//!
//! 冒号中文英文都认（`vbit:` 与 `vbit：` 一样），分号也认 —— 键盘上冒号就是 Shift + `;`，
//! 有些布局送到缓冲区的是分号那一下。不写子模式、或前缀认不出来时走缺省计算，
//! 与之前的行为完全一致：`v1+2`、`v123`、`v1998-3-8` 一个都不受影响。

use crate::shortcut::bitwise::BitWidth;
use crate::shortcut::{TIMESTAMP_KEYWORD, UUID_KEYWORD};

/// 子模式。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode<'a> {
    /// 生成一个 UUID（`vuuid`、`vuuid;`）：无参，不打分隔符也认。
    Uuid,

    /// 时间戳（`vts`、`vts;`、`vts;2026-11-05`）：参数是时刻，空就是此刻。
    Timestamp {
        /// 冒号之后的时刻原文；空串是此刻。
        at: &'a str,
    },

    /// 位运算，位宽与符号由子模式名给。
    Bit {
        /// 位宽与符号。
        width: BitWidth,

        /// 冒号之后的表达式原文。
        body: &'a str,
    },
}

/// 拆子模式。`body` 是表达式键之后的部分（不含 `v`）。
///
/// 没有冒号时返回 `None`：那是缺省计算，交给原来的四则 / 日期 / 时间戳那套。
pub fn parse(body: &str) -> Option<Mode<'_>> {
    match split_colon(body) {
        Some((name, rest)) => named(name, rest),
        // 无参子模式不用打分隔符：`vuuid` 与 `vuuid;` 等价，`vts` 与 `vts;` 等价
        None => named(body.trim_end_matches([';', '；']), ""),
    }
}

/// 按名字认子模式。`uuid` 与 `ts` 无参（`ts` 的参数是可选时刻），其余要参数。
fn named<'a>(name: &str, rest: &'a str) -> Option<Mode<'a>> {
    if name == UUID_KEYWORD {
        return Some(Mode::Uuid);
    }
    if name == TIMESTAMP_KEYWORD {
        return Some(Mode::Timestamp { at: rest.trim() });
    }
    let width = parse_bit_width(name)?;
    Some(Mode::Bit { width, body: rest })
}

/// 认 `bit` / `bitu` / `bitN` / `bitNu`；别的名字返回 `None`。
fn parse_bit_width(name: &str) -> Option<BitWidth> {
    let digits = name.strip_prefix("bit")?;
    // `bitu`：64 位无符号
    if digits == "u" {
        return Some(BitWidth {
            bits: 64,
            signed: false,
        });
    }
    let (number, signed) = match digits.strip_suffix('u') {
        Some(number) => (number, false),
        None => (digits, true),
    };
    if number.is_empty() {
        return Some(BitWidth { bits: 64, signed });
    }
    let bits: u32 = number.parse().ok()?;
    matches!(bits, 8 | 16 | 32 | 64 | 128).then_some(BitWidth { bits, signed })
}

/// 在第一个冒号处切开。半角冒号、全角冒号、半角分号、全角分号都算 ——
/// 键盘上冒号就是 Shift + `;`，不少布局 / 输入框里送到缓冲区的是分号那一下。
fn split_colon(body: &str) -> Option<(&str, &str)> {
    let mut cut = None;
    for (index, c) in body.char_indices() {
        if matches!(c, ':' | '：' | ';' | '；') {
            cut = Some((index, c.len_utf8()));
            break;
        }
    }
    let (index, width) = cut?;
    let name = &body[..index];
    // 子模式名只能是字母数字，空的不算（`v:abc` 是缺省计算里带冒号，交给下面按普通算式判死）
    if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return None;
    }
    Some((name, &body[index + width..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_widths() {
        let cases = [
            ("bit:1<<3", 64, true),
            ("bitu:1<<3", 64, false),
            ("bit8:0xff", 8, true),
            ("bit8u:0xff", 8, false),
            ("bit16:65535", 16, true),
            ("bit32:0xdead", 32, true),
            ("bit128:1<<100", 128, true),
        ];
        for (text, bits, signed) in cases {
            match parse(text).unwrap() {
                Mode::Bit { width, .. } => {
                    assert_eq!(width.bits, bits, "{text}");
                    assert_eq!(width.signed, signed, "{text}");
                }
                other => panic!("期望 Bit，得到 {other:?}"),
            }
        }
        assert!(parse("bit15:1").is_none());
    }

    #[test]
    fn full_width_colon_and_semicolon_are_accepted() {
        assert!(matches!(parse("bit8：0x1ff"), Some(Mode::Bit { .. })));
        assert!(matches!(parse("bit8;0x1ff"), Some(Mode::Bit { .. })));
        assert!(matches!(parse("bit8；0x1ff"), Some(Mode::Bit { .. })));
    }

    #[test]
    fn paramless_names_need_no_separator() {
        assert!(matches!(parse("uuid"), Some(Mode::Uuid)));
        assert!(matches!(parse("uuid;"), Some(Mode::Uuid)));
        assert!(matches!(parse("ts"), Some(Mode::Timestamp { at: "" })));
        assert!(matches!(parse("ts;"), Some(Mode::Timestamp { at: "" })));
        match parse("ts;2026-11-05") {
            Some(Mode::Timestamp { at }) => assert_eq!(at, "2026-11-05"),
            other => panic!("期望 Timestamp，得到 {other:?}"),
        }
        // 不是子模式的仍然不是
        assert!(parse("1+2").is_none());
        assert!(parse("uuidx").is_none());
    }
    #[test]
    fn unknown_prefix_is_not_a_mode() {
        assert!(parse("1+2").is_none());
        assert!(parse(":1+2").is_none());
        assert!(parse("foo:bar").is_none());
    }
}
