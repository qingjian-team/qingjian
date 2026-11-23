//! `v` 模式下的子模式：`v<名字>:<语句>`。
//!
//! 所有子模式在 `SPECS` 这一张表里登记：名字、别名、名字敲全时的用法提示、参数里收哪些键。
//! 名字认不认、按键收不收、提示说什么，都由这张表推出来，不在别处各写一遍。
//!
//! 加一个子模式只要动三处，且都挤在一起：
//!
//! 1. [`Kind`] 加一个种类
//! 2. `SPECS` 加一行
//! 3. `shortcut::mode_forms` 加一个分支算候选
//!
//! - [`parse`]：拆出子模式
//! - [`is_name_prefix`]：`v` 之后敲的字母是不是某个名字的开头（决定名字能不能敲进去）
//! - [`accepts_key`]：子模式里某个字符该不该进缓冲区（`None` = 与子模式无关，交给算式判断）
//! - [`hint`]：名字敲全、参数还没填时的一句用法
//!
//! 冒号中文英文都认（`vbit:` 与 `vbit：` 一样），分号也认 —— 键盘上冒号就是 Shift + `;`，
//! 有些布局送到缓冲区的是分号那一下。不写子模式、或前缀认不出来时走缺省计算，
//! 与之前的行为完全一致：`v1+2`、`v123`、`v1998-3-8` 一个都不受影响。

use crate::shortcut::bitwise::BitWidth;
use crate::shortcut::{TIMESTAMP_KEYWORD, UUID_KEYWORD};

/// 位运算子模式的名字。
pub const BIT_KEYWORD: &str = "bit";

/// 进制转换子模式的名字。
pub const BASE_KEYWORD: &str = "base";

/// 统计子模式的名字。
pub const STAT_KEYWORD: &str = "stat";

/// 变换子模式的名字（归一化、对数、差分、平滑、分箱…）。
pub const TRANS_KEYWORD: &str = "trans";

/// 颜色子模式的名字（`vcol` 与 `vcolor` 都认）。
pub const COLOR_KEYWORD: &str = "col";

/// 子模式的种类。`SPECS` 与 `mode_forms` 都按它分派。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Bit,
    Uuid,
    Timestamp,
    Base,
    Stat,
    Trans,
    Color,
}

/// 认出来的子模式：种类 + 参数原文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mode<'a> {
    pub kind: Kind,

    /// 冒号之后的参数原文。
    pub args: &'a str,
}

/// 一条子模式的登记项。
struct Spec {
    kind: Kind,

    /// 名字（`bit`、`ts`、`uuid`、`col`）。别名各占一行，见下。
    name: &'static str,

    /// 参数里除字母数字外还收哪些符号。
    keys: &'static [char],

    /// 名字敲全、参数还空着时的一句用法；不需要提示的名字给空串。
    hint: &'static str,
}

/// 全部子模式。加一个在这里登记，`Kind` 与 `mode_forms` 跟上即可。
///
/// 别名（`vcolor`）与主名一样单独占一行 —— 表是一维的，多写一行比再记一套「别名」规则省事。
const SPECS: [Spec; 8] = [
    Spec {
        kind: Kind::Bit,
        name: BIT_KEYWORD,
        keys: &[
            '+', '-', '*', '/', '%', '^', '&', '|', '~', '<', '>', '(', ')',
        ],
        hint: "vbit：算式　位宽写在参数里：vbit;8u;1-2（8 / 16 / 32 / 128，加 u 按无符号算）",
    },
    Spec {
        kind: Kind::Uuid,
        name: UUID_KEYWORD,
        keys: &[],
        hint: "",
    },
    Spec {
        kind: Kind::Timestamp,
        name: TIMESTAMP_KEYWORD,
        keys: &['-', '.', '/', ':'],
        hint: "",
    },
    Spec {
        kind: Kind::Base,
        name: BASE_KEYWORD,
        keys: &['.', '-'],
        hint: "vbase：数;进制　十进制换成 2–36 进制",
    },
    Spec {
        kind: Kind::Stat,
        name: STAT_KEYWORD,
        keys: &['.', '-'],
        hint: "vstat：数;数;…　给平均数、中位数、四分位数、方差、极差",
    },
    Spec {
        kind: Kind::Trans,
        name: TRANS_KEYWORD,
        keys: &['.', '-'],
        hint: "vtrans：算子;数;数;…　sort desc minmax z log log10 sqrt diff smooth median low high bin uniq shuffle",
    },
    Spec {
        kind: Kind::Color,
        name: COLOR_KEYWORD,
        keys: &['#', '(', ')', ',', '.', '%', '-', '/', ':'],
        hint: "vcol：颜色　#ff8800 / rgb(255,136,0) / hsl(32,100%,50%) / 255;136;0 / 5000k",
    },
    Spec {
        kind: Kind::Color,
        name: "color",
        keys: &['#', '(', ')', ',', '.', '%', '-', '/', ':'],
        hint: "vcolor：同 vcol",
    },
];

impl Mode<'_> {
    /// 这条子模式带的参数原文（`uuid` 没有，给空串）。
    pub fn args(&self) -> &str {
        self.args
    }

    /// 换一份参数（剪切板里的内容顶上来时用），种类不动。
    pub fn with_args(self, args: &str) -> Mode<'_> {
        Mode {
            kind: self.kind,
            args,
        }
    }

    /// 这个名字还要不要参数：`uuid` 不要（打不打分隔符都行），其余要。
    pub fn wants_argument(&self) -> bool {
        self.kind != Kind::Uuid
    }

    /// 语句里这个字符该不该进缓冲区。
    pub fn accepts(&self, c: char) -> bool {
        if !self.wants_argument() {
            return false;
        }
        // 分隔符一直能敲：`vstat;1;2`、`vbit;8u;1-2` 都靠它接下一段
        if matches!(c, ';' | '；') {
            return true;
        }
        let Some(spec) = spec_of(self.kind) else {
            return false;
        };
        c.is_ascii_alphanumeric() || spec.keys.contains(&c)
    }
}

/// 按种类找登记项（同一种类的主名与别名取第一条，`keys` 一致）。
fn spec_of(kind: Kind) -> Option<&'static Spec> {
    SPECS.iter().find(|spec| spec.kind == kind)
}

/// 按名字找登记项。
fn spec_by_name(name: &str) -> Option<&'static Spec> {
    SPECS.iter().find(|spec| spec.name == name)
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

/// 按名字认子模式。
fn named<'a>(name: &str, rest: &'a str) -> Option<Mode<'a>> {
    let spec = spec_by_name(name)?;
    Some(Mode {
        kind: spec.kind,
        args: rest,
    })
}

/// 子模式名敲全、参数还没填时的一句用法；别的输入为 `None`。
///
/// `vtrans`、`vstat` 这类要参数，光敲名字时候选是空的，不说一句就是一片空白。
/// 参数一开头就不给了：再敲一个键就有内容，提示反而挡路。
pub fn hint(body: &str) -> Option<&'static str> {
    let name = body.trim_end_matches([';', '；']);
    if name.contains([';', '；', ':']) {
        return None;
    }
    let spec = spec_by_name(name)?;
    (!spec.hint.is_empty()).then_some(spec.hint)
}

/// `vbit` 的参数：开头一段可能是位宽（`8` / `16` / `32` / `64` / `128`，后缀 `u` 表示无符号），
/// 后面才是算式。没写位宽时按缺省 64 位有符号算 —— `vbit;1+1` 的老写法照旧。
///
/// `vbit;8` 这种只有一段的按算式算（求值就是 8），当位宽得写成 `vbit;8;7`。
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

/// `body` 是不是某个子模式名的开头：`b`、`bi`、`c`、`colo` 这些。
///
/// 按键层据此决定 `v` 之后敲的要不要进缓冲区 —— 不收的话 `vbit` 这个名字本身
/// 就敲不进去（`b` 不是算式字符）。名字敲全之后不算前缀，那时数字要留给选词
/// （`vts` 有四条候选，敲 `1` 是选它）。
pub fn is_name_prefix(body: &str) -> bool {
    if body.is_empty() || !body.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return false;
    }
    SPECS
        .iter()
        .any(|spec| spec.name.len() > body.len() && spec.name.starts_with(body))
}

/// 表达式模式下这个字符进不进缓冲区。`None` 表示与子模式无关，交给算式那边判断。
///
/// 三种情形：
/// - **名字还没敲完**（`v`、`vb`、`vbi`）：字母与分隔符都收，数字不收（`v123` 还是算式）
/// - **名字敲完、还没敲分隔符**（`vts`、`vbit`）：只收分隔符，数字留给选词
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
    if spec_by_name(body).is_some() {
        return Some(separator);
    }
    if is_name_prefix(body) {
        return Some(c.is_ascii_alphanumeric() || separator);
    }
    None
}

/// 在第一个冒号处切开。半角冒号、全角冒号、半角分号、全角分号都算 ——
/// 键盘上冒号就是 Shift + `;`，有些布局送到缓冲区的是分号那一下。
fn split_colon(body: &str) -> Option<(&str, &str)> {
    body.char_indices()
        .find(|(_, c)| matches!(c, ':' | '：' | ';' | '；'))
        .map(|(at, c)| (&body[..at], &body[at + c.len_utf8()..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_prefixes() {
        assert!(is_name_prefix("b"));
        assert!(is_name_prefix("bi"));
        assert!(is_name_prefix("c"));
        assert!(is_name_prefix("colo"));
        // 名字敲全就不算前缀了：那时由 accepts_key 按子模式判
        assert!(!is_name_prefix("bit"));
        // `col` 敲全了，但它仍是别名 `color` 的开头，得让用户接着敲 `o`
        assert!(is_name_prefix("col"));
        assert!(!is_name_prefix("color"));
        assert!(!is_name_prefix("123"));
    }

    #[test]
    fn hint_only_while_the_name_stands_alone() {
        assert!(hint("trans").is_some_and(|text| text.contains("minmax")));
        assert!(hint("trans;").is_some());
        assert!(hint("stat").is_some());
        assert!(hint("base").is_some());
        assert!(hint("bit").is_some());
        assert!(hint("col").is_some_and(|text| text.contains("rgb(")));
        assert!(hint("color").is_some());
        // 名字没敲全、或已经在填参数，都不提示
        assert!(hint("co").is_none());
        assert!(hint("trans;lo").is_none());
        // 不需要参数的名字不给
        assert!(hint("ts").is_none());
        assert!(hint("uuid").is_none());
        assert!(hint("").is_none());
        assert!(hint("datetime").is_none());
    }

    #[test]
    fn which_keys_reach_the_buffer() {
        assert_eq!(accepts_key("", 'b'), Some(true));
        assert_eq!(accepts_key("", '1'), None);
        assert_eq!(accepts_key("bi", 't'), Some(true));
        assert_eq!(accepts_key("bi", ':'), Some(true));
        // 名字敲完：只收分隔符，数字留给选词
        assert_eq!(accepts_key("bit", '8'), Some(false));
        assert_eq!(accepts_key("bit", ';'), Some(true));
        assert_eq!(accepts_key("ts", '1'), Some(false));
        assert_eq!(accepts_key("ts", ';'), Some(true));
        // 进了参数按子模式来
        assert_eq!(accepts_key("bit:", '<'), Some(true));
        assert_eq!(accepts_key("bit:", '&'), Some(true));
        assert_eq!(accepts_key("ts;2024", '<'), Some(false));
        assert_eq!(accepts_key("ts;2024", ':'), Some(true));
        // 颜色：hex 的 `#` 与逗号要收，`+` 不收
        assert_eq!(accepts_key("col;#", '#'), Some(true));
        assert_eq!(accepts_key("col;rgb(1,2,3)", ','), Some(true));
        assert_eq!(accepts_key("col;1", '+'), Some(false));
        assert_eq!(accepts_key("123", '+'), None);
    }

    #[test]
    fn every_registered_name_takes_a_separator() {
        for spec in SPECS.iter() {
            let name = spec.name;
            assert!(parse(name).is_some(), "`{name}` 认不出来");
            if spec.kind == Kind::Uuid {
                continue; // 无参子模式，分隔符之后不再收东西
            }
            assert_eq!(
                accepts_key(name, ';'),
                Some(true),
                "`v{name};` 的分号进不了缓冲区"
            );
        }
    }

    #[test]
    fn parses_names_and_bit_args() {
        assert_eq!(parse("col;#ff8800").map(|m| m.kind), Some(Kind::Color));
        assert_eq!(parse("color;#ff8800").map(|m| m.kind), Some(Kind::Color));
        assert_eq!(parse("uuid").map(|m| m.kind), Some(Kind::Uuid));
        assert!(parse("1+2").is_none());

        let cases = [
            ("", 64, true),
            ("1<<3", 64, true),
            ("8u;1-2", 8, false),
            ("8;0xff", 8, true),
            ("128u;1", 128, false),
            // 只有一段按算式算，不当位宽
            ("8", 64, true),
            // 认不出来的位宽当算式，缺省位宽
            ("7;1", 64, true),
        ];
        for (args, bits, signed) in cases {
            let (width, _) = bit_args(args);
            assert_eq!((width.bits, width.signed), (bits, signed), "{args}");
        }
    }
}
