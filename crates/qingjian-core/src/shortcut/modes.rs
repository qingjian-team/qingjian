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

/// 位运算子模式的名字。`bit` 这一族（`bitu` / `bit8` / `bit128u`）都从它派生。
pub const BIT_KEYWORD: &str = "bit";

/// 进制转换子模式的名字。
pub const BASE_KEYWORD: &str = "base";

/// 统计子模式的名字。
pub const STAT_KEYWORD: &str = "stat";

/// 颜色子模式的名字（`vcol` 与 `vcolor` 都认）。
pub const COLOR_KEYWORD: &str = "col";

/// 变换子模式的名字（归一化、对数、差分、平滑、分箱…）。
pub const TRANS_KEYWORD: &str = "trans";

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

    /// 位运算：`vbit;8u;1-2`。位宽写在参数开头，没写按缺省 64 位有符号算。
    Bit {
        /// 冒号之后的参数原文（可能以位宽开头）。
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

    /// 变换：`vtrans;log;1;2;4` 对一串数做归一化、取对数、差分、平滑、分箱这些。
    Trans {
        /// 冒号之后的参数原文（算子名在前，其余是数）。
        args: &'a str,
    },

    /// 颜色：`vcol;#ff8800` 给各种颜色写法。
    Color {
        /// 冒号之后的参数原文。
        args: &'a str,
    },
}

impl<'a> Mode<'a> {
    /// 这条子模式带的参数原文（`uuid` 没有，给空串）。
    pub fn args(&self) -> &'a str {
        match self {
            Self::Uuid => "",
            Self::Timestamp { at } => at,
            Self::Bit { body, .. } => body,
            Self::Base { args }
            | Self::Stat { args }
            | Self::Trans { args }
            | Self::Color { args } => args,
        }
    }

    /// 换一份参数（剪切板里的内容顶上来时用），名字与位宽都不动。
    pub fn with_args(self, args: &'a str) -> Self {
        match self {
            Self::Uuid => self,
            Self::Timestamp { .. } => Self::Timestamp { at: args },
            Self::Bit { .. } => Self::Bit { body: args },
            Self::Base { .. } => Self::Base { args },
            Self::Stat { .. } => Self::Stat { args },
            Self::Trans { .. } => Self::Trans { args },
            Self::Color { .. } => Self::Color { args },
        }
    }

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
            // 进制、统计与变换：数字、字母（进制名 / 算子名）、小数、负号与分号
            Self::Base { .. } | Self::Stat { .. } | Self::Trans { .. } => {
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
            // 颜色：hex 的 `#`、函数式的 `rgb(…)`、逗号与百分号都要进得来
            Self::Color { .. } => {
                c.is_ascii_alphanumeric()
                    || matches!(
                        c,
                        '#' | '(' | ')' | ',' | '.' | '%' | '-' | '/' | ';' | '；' | '：' | ':'
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
const SPECS: [Spec; 8] = [
    Spec {
        name: BIT_KEYWORD,
        numbered: false,
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
        name: BASE_KEYWORD,
        numbered: false,
    },
    Spec {
        name: STAT_KEYWORD,
        numbered: false,
    },
    Spec {
        name: TRANS_KEYWORD,
        numbered: false,
    },
    Spec {
        name: COLOR_KEYWORD,
        numbered: false,
    },
    Spec {
        name: "color",
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

/// 子模式名敲全、还没填参数时的一句用法；别的输入为 `None`。
///
/// `vtrans`、`vstat` 这类要参数，光敲名字时候选是空的，不说一句就是一片空白。
/// 参数一开头就不给了：再敲一个键就有内容，提示反而挡路。
pub fn hint(body: &str) -> Option<&'static str> {
    // 名字敲全、参数还空着：末尾的分隔符不算参数，名字里再出现分隔符就是已经在填了
    let name = body.trim_end_matches([';', '；']);
    if name.contains([';', '；', ':']) {
        return None;
    }
    match name {
        "bit" => Some("vbit：算式　位宽写 bit8 / bit16 / bit32，加 u 按无符号算（vbit8u;1-2）"),
        "base" => Some("vbase：数;进制　十进制换成 2–36 进制"),
        "stat" => Some("vstat：数;数;…　给平均数、中位数、四分位数、方差、极差"),
        "trans" => Some(
            "vtrans：算子;数;数;…　minmax z log log10 sqrt diff smooth median low high bin uniq shuffle",
        ),
        _ => None,
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
    if name == BASE_KEYWORD {
        return Some(Mode::Base { args: rest });
    }
    if name == STAT_KEYWORD {
        return Some(Mode::Stat { args: rest });
    }
    if name == COLOR_KEYWORD || name == "color" {
        return Some(Mode::Color { args: rest });
    }
    if name == TRANS_KEYWORD {
        return Some(Mode::Trans { args: rest });
    }
    if name == BIT_KEYWORD {
        return Some(Mode::Bit { body: rest });
    }
    None
}

/// `vbit` 的参数：开头一段可能是位宽（`8` / `16` / `32` / `64` / `128`，后缀 `u` 表示无符号），
/// 后面才是算式。没写位宽时按缺省 64 位有符号算 —— `vbit;1+1` 的老写法照旧。
///
/// `vbit;8` 这种只有一段的按算式算（求值就是 8），当位宽要写成 `vbit;8;7`。
pub fn bit_args(args: &str) -> (BitWidth, &str) {
    let Some((head, rest)) = args.split_once([';', '；']) else {
        return (BitWidth::DEFAULT, args);
    };
    if rest.trim().is_empty() {
        return (BitWidth::DEFAULT, args);
    }
    match width_of_arg(head.trim()) {
        Some(width) => (width, rest),
        None => (BitWidth::DEFAULT, args),
    }
}

/// 位宽那一段：`8`、`16u`、`128`、`u`（64 位无符号）。
fn width_of_arg(text: &str) -> Option<BitWidth> {
    let (digits, signed) = match text.strip_suffix(['u', 'U']) {
        Some(rest) => (rest, false),
        None => (text, true),
    };
    if digits.is_empty() {
        return (!signed).then_some(BitWidth {
            bits: 64,
            signed: false,
        });
    }
    let bits: u32 = digits.parse().ok()?;
    matches!(bits, 8 | 16 | 32 | 64 | 128).then_some(BitWidth { bits, signed })
}

/// 带参数子模式的参数：按分隔符切开，空段丢掉。
///
/// `vbase;128;16` 与 `vstat;1;2;3` 都走这里，个数由各子模式自己校验。
///
/// 分隔符比键盘上敲得出来的宽：空格、制表、换行、半角与全角逗号也算。键盘上敲不进这些
/// （空格是上屏键、逗号是标点），所以对打字没有影响；从剪切板粘进来的一列数常是
/// 空格或换行分隔的，多认几种才用得起来。
pub fn split_args(args: &str) -> Vec<&str> {
    args.split([';', '；', ',', '，', ' ', '\t', '\n', '\r', '\u{3000}'])
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
        false
    })
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

    /// 位宽写在参数里：`vbit;8u;1-2`。缺省 64 位有符号，`u` 后缀是无符号。
    #[test]
    fn bit_widths() {
        let cases = [
            ("", 64, true),
            ("1<<3", 64, true),
            ("8u;1-2", 8, false),
            ("8;0xff", 8, true),
            ("128u;1", 128, false),
            // `vbit;8` 只有一段，按算式算（求值就是 8），不当位宽
            ("8", 64, true),
            // 认不出来的位宽当算式，缺省位宽
            ("7;1", 64, true),
        ];
        for (args, bits, signed) in cases {
            let (width, _) = bit_args(args);
            assert_eq!((width.bits, width.signed), (bits, signed), "{args}");
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
        for body in ["b", "bi", "u", "uu", "t"] {
            assert!(is_name_prefix(body), "{body} 该算前缀");
        }
        // 位宽改成写在参数里，名字后面不再接数字
        assert!(!is_name_prefix("bit8"));
        assert!(!is_name_prefix("bit128u"));
        // 终结名敲完了，数字留给选词
        assert!(!is_name_prefix("ts"));
        assert!(!is_name_prefix("uuid"));
        // 纯数字不是前缀
        assert!(!is_name_prefix("123"));
    }

    /// 名字敲全、还没填参数时给一句用法；一填参数就不再提示。
    #[test]
    fn hint_only_while_the_name_stands_alone() {
        assert!(hint("trans").is_some_and(|text| text.contains("minmax")));
        // 末尾的分隔符不算参数
        assert!(hint("trans;").is_some_and(|text| text.contains("minmax")));
        assert!(hint("stat").is_some());
        assert!(hint("base").is_some());
        assert!(hint("bit").is_some());
        // 一填参数、或名字带宽度，就不再给
        assert!(hint("trans;lo").is_none());
        assert!(hint("bit8").is_none());
        // 别的词不是子模式
        assert!(hint("").is_none());
        assert!(hint("datetime").is_none());
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
        // `bit` 敲完了：位宽也写在参数里，数字不进缓冲区（那是算式或选词）
        assert_eq!(accepts_key("bit", '8'), Some(false));
        assert_eq!(accepts_key("bit", ';'), Some(true));
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

    /// 每个登记项都得能敲出分隔符、都得认得出名字。
    ///
    /// 这条盯的是「登记漏了」这类错：少了 SPECS 里那一行，`vcol;` 的分号会被当标点上屏，
    /// 看着像输入法坏了。新加子模式时它自己就会催你补上。
    #[test]
    fn every_registered_name_takes_a_separator() {
        // 别名单独一行登记（`vcolor` 与 `vcol` 各占一行），这里遍历所有登记名
        for spec in SPECS.iter() {
            for name in std::iter::once(spec.name) {
                assert!(
                    matches!(parse(name), Some(_)),
                    "`{name}` 认不出来（SPECS 那一行写错了？）"
                );
                if name == UUID_KEYWORD {
                    continue; // 无参子模式，分隔符之后不再收东西
                }
                assert_eq!(
                    accepts_key(name, ';'),
                    Some(true),
                    "`v{name};` 的分号进不了缓冲区"
                );
            }
        }
    }
}
