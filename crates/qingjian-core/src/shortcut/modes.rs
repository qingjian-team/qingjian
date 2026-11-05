//! `v` 模式下的子模式：`v<名字>:<语句>`。
//!
//! 所有子模式在这里统一登记 —— 加一个只需动 `Mode`、`SPECS` 与 `mode_forms` 三处，
//! 名字认不认、按键收不收都由这张表推出来，别处不再各写一套：
//!
//! - [`parse`]：拆出子模式
//! - [`is_name_prefix`]：`v` 之后敲的字母是不是某个名字的开头（决定名字能不能敲进去）
//! - [`accepts_key`]：子模式里某个字符该不该进缓冲区（`None` = 与子模式无关，交给算式判断）
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

    /// 进制转换：`vbase;128;16` 把 128 从十进制换成十六进制。
    Base {
        /// 冒号之后的参数原文，按分号切开再各自校验。
        args: &'a str,
    },

    /// 统计：`vstat;1;2;3` 给个数、和、平均、中位数这些。
    Stat {
        /// 冒号之后的参数原文。
        args: &'a str,
    },
}

impl Mode<'_> {
    /// 这个名字还要不要参数：`uuid` 不要（打不打分隔符都行），其余要。
    pub fn wants_argument(&self) -> bool {
        !matches!(self, Self::Uuid)
    }

    /// 语句里这个字符该不该进缓冲区。
    pub fn accepts(&self, c: char) -> bool {
        match self {
            // `uuid` 没有参数，分隔符之后再敲什么都进不去
            Self::Uuid => false,
            // 时刻：数字与几种分隔符。空格不行 —— 那是上屏键
            Self::Timestamp { .. } => {
                c.is_ascii_digit() || matches!(c, '-' | '.' | '/' | ':' | ';' | '；')
            }
            // 进制与统计：数字、字母（进制名）、小数、负号与分号
            Self::Base { .. } | Self::Stat { .. } => {
                c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ';' | '；')
            }
            // 位运算照 C++：`<<` `>>` `&` `|` `^` `~` `%`，还有写出来的英文（`shl` `and`）
            Self::Bit { .. } => {
                c.is_ascii_alphanumeric()
                    || matches!(
                        c,
                        '+' | '-' | '*' | '/' | '%' | '^' | '&' | '|' | '~' | '<' | '>' | '(' | ')'
                    )
            }
        }
    }
}

/// 一条子模式的登记项。
struct Spec {
    /// 名字（`bit`、`ts`、`uuid`）。
    name: &'static str,

    /// 名字后面还能接位宽数字（`bit8`、`bit128u`）。终结名（`ts`、`uuid`）为 `false`：
    /// 它们后面敲数字是选词，不是名字的一部分。
    numbered: bool,
}

/// 全部子模式。加一个在这里登记，`Mode` 与 `mode_forms` 跟上即可。
const SPECS: [Spec; 5] = [
    Spec {
        name: "bit",
        numbered: true,
    },
    Spec {
        name: UUID_KEYWORD,
        numbered: false,
    },
    Spec {
        name: TIMESTAMP_KEYWORD,
        numbered: false,
    },
    Spec {
        name: "base",
        numbered: false,
    },
    Spec {
        name: "stat",
        numbered: false,
    },
];

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
    if name == "base" {
        return Some(Mode::Base { args: rest });
    }
    if name == "stat" {
        return Some(Mode::Stat { args: rest });
    }
    let width = parse_bit_width(name)?;
    Some(Mode::Bit { width, body: rest })
}

/// 带参数子模式的参数：按分号切，全角也认，空段丢掉。
///
/// `vbase;128;16` 与 `vstat;1;2;3` 都走这里，个数由各子模式自己校验。
pub fn split_args(args: &str) -> Vec<&str> {
    args.split([';', '；'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect()
}

/// `body` 是不是某个子模式名的开头：`b`、`bi`、`bit`、`bit8`、`uu`、`t` 这些。
///
/// 按键层据此决定 `v` 之后敲的要不要进缓冲区 —— 不收的话 `vbit` 这个名字本身
/// 就敲不进去（`b` 不是算式字符）。终结名（`ts`、`uuid`）敲完之后不再算前缀，
/// 那时数字要留给选词（`vts` 有四条候选，敲 `1` 是选它）。
pub fn is_name_prefix(body: &str) -> bool {
    if body.is_empty() || !body.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return false;
    }
    SPECS.iter().any(|spec| {
        // 名字还没敲完：`t` 之于 `ts`
        if spec.name.len() > body.len() && spec.name.starts_with(body) {
            return true;
        }
        // 名字敲完了、还能接位宽：`bit` 之后的 `8`、`bit128` 之后的 `u`。
        // 只放行合法位宽的前缀，`bit7`、`bit256`、`bit8x` 这种敲错了的不进缓冲区
        // —— 否则用户会看到一串空候选，不知道为什么。`strip_prefix` 不成立说明
        // 这条名字根本不匹配（`ts` 之于 `bit`），不能当成「刚敲完」放行。
        match spec
            .numbered
            .then(|| body.strip_prefix(spec.name))
            .flatten()
        {
            Some(digits) => is_width_prefix(digits),
            None => false,
        }
    })
}

/// `bit` 后面的那段是不是合法位宽的开头：`8`、`1`（16 / 128 的前缀）、`12`、`128u`。
///
/// 空串算（刚敲完 `bit`）。`7`、`256`、`8x` 这些不是，按键层据此拦住。
fn is_width_prefix(digits: &str) -> bool {
    let digits = digits.strip_suffix('u').unwrap_or(digits);
    digits.is_empty()
        || ["8", "16", "32", "64", "128"]
            .iter()
            .any(|width| width.starts_with(digits))
}

/// 表达式模式下这个字符进不进缓冲区。`None` 表示与子模式无关，交给算式那边判断。
///
/// 三种情形：
/// - **名字还没敲完**（`v`、`vb`、`vbi`）：字母与分隔符都收，数字不收（`v123` 还是算式）
/// - **名字敲完、还没敲分隔符**（`vts`、`vbit`）：分隔符收，其余看子模式 ——
///   `vbit` 后面还要接位宽所以数字也收，`vts` 后面数字是选词键
/// - **已经敲了分隔符**：按那个子模式的参数规则来
pub fn accepts_key(body: &str, c: char) -> Option<bool> {
    let separator = matches!(c, ':' | '：' | ';' | '；');
    if let Some((name, rest)) = split_colon(body) {
        return named(name, rest).map(|mode| mode.accepts(c));
    }
    if body.is_empty() {
        // `v` 之后刚开始：字母（敲名字）与分隔符（`v;` 空参数）都收
        return (c.is_ascii_alphabetic() || separator).then_some(true);
    }
    if !body.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return None;
    }
    // 名字正好敲完了（`ts`、`uuid`、`bit`）：终结名只再收分隔符，数字留给选词
    // （`vts` 有四条候选，敲 `1` 是选第一条）；`bit` 还能接位宽所以数字也收。
    if SPECS.iter().any(|spec| spec.name == body) {
        let extending = SPECS
            .iter()
            .any(|spec| spec.numbered && body == spec.name && c.is_ascii_alphanumeric());
        return Some(separator || extending);
    }
    if is_name_prefix(body) {
        // 名字还没敲完：字母数字与分隔符都收
        return Some(c.is_ascii_alphanumeric() || separator);
    }
    None
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
        ];
        for (body, bits, signed) in cases {
            match parse(body) {
                Some(Mode::Bit { width, .. }) => {
                    assert_eq!((width.bits, width.signed), (bits, signed), "{body}");
                }
                other => panic!("{body} 该认成 Bit，得到 {other:?}"),
            }
        }
    }

    #[test]
    fn full_width_colon_and_semicolon_are_accepted() {
        assert!(matches!(parse("bit：1"), Some(Mode::Bit { .. })));
        assert!(matches!(parse("bit;1"), Some(Mode::Bit { .. })));
        assert!(matches!(parse("bit；1"), Some(Mode::Bit { .. })));
    }

    #[test]
    fn paramless_names_need_no_separator() {
        assert_eq!(parse("uuid"), Some(Mode::Uuid));
        assert_eq!(parse("uuid;"), Some(Mode::Uuid));
        assert!(matches!(parse("ts"), Some(Mode::Timestamp { at: "" })));
        assert!(matches!(parse("ts;"), Some(Mode::Timestamp { at: "" })));
    }

    #[test]
    fn unknown_prefix_is_not_a_mode() {
        assert!(parse("1+2").is_none());
        assert!(parse(":1+2").is_none());
        assert!(parse("foo:bar").is_none());
    }

    /// `v` 之后敲名字的过程要能进；终结名敲完之后数字让给选词。
    #[test]
    fn name_prefixes_and_extensions() {
        // 名字还没敲完
        for body in ["b", "bi", "bit", "u", "uu", "t"] {
            assert!(is_name_prefix(body), "{body} 该算前缀");
        }
        // `bit` 还能接位宽
        assert!(is_name_prefix("bit"));
        assert!(is_name_prefix("bit8"));
        assert!(is_name_prefix("bit128u"));
        // 终结名敲完了，数字留给选词
        assert!(!is_name_prefix("ts"));
        assert!(!is_name_prefix("uuid"));
        // 纯数字不是前缀
        assert!(!is_name_prefix("123"));
    }

    /// 按键放行：名字阶段收字母数字与分隔符，参数阶段按子模式。
    #[test]
    fn which_keys_reach_the_buffer() {
        // `v` 之后开始敲名字：字母进（敲名字），数字不进 —— 那是算式（`v1+2`）
        assert_eq!(accepts_key("", 'b'), Some(true));
        assert_eq!(accepts_key("", '1'), None);
        // 名字还没敲完
        assert_eq!(accepts_key("bi", 't'), Some(true));
        assert_eq!(accepts_key("bi", ':'), Some(true));
        // `bit` 接位宽
        assert_eq!(accepts_key("bit", '8'), Some(true));
        // `ts` 敲完了，数字是选词键（明确不收，不是「不管」）
        assert_eq!(accepts_key("ts", '1'), Some(false));
        assert_eq!(accepts_key("ts", ';'), Some(true));
        // 进了参数：位运算收运算符，时刻只收数字与几种分隔符
        assert_eq!(accepts_key("bit:", '<'), Some(true));
        assert_eq!(accepts_key("bit:", '&'), Some(true));
        assert_eq!(accepts_key("ts;2024", '<'), Some(false));
        assert_eq!(accepts_key("ts;2024", ':'), Some(true));
        // 不是子模式
        assert_eq!(accepts_key("123", '+'), None);
    }
}
