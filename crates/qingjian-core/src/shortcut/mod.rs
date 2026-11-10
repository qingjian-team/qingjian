//! 快捷候选（搜狗「v 模式」的那一套）：不查词库、由输入本身直接算出来的候选。
//!
//! - `rq` / `sj` / `xq`：今天的日期、现在的时间、星期几，插在本地候选第二位起；打全拼出了「日期 / 时间 / 星期」
//!   这几个词（`riqi`、九宫格的 `7474`）时同样跟在这个词后面（[`keyword_forms`]）。
//! - 表达式键（缺省 `v`）开头进表达式模式：`v1+2` 出 `3` 与 `1+2=3`，`v123` 出中文数字与金额（小写与大写），
//!   `v123.5` 出小数读法与金额，同样各有小写与大写（一百二十三元五角 / 壹佰贰拾叁元伍角）；
//!   `v1998-3-8`（四位年份 + 月 + 日，分隔符 `-` / `/` / `.`）出中文日期 `一九九八年三月八日` 与
//!   `1998年3月8日` / `1998-03-08`，年份不是四位或月日不合法时回落到算式（见 [`input_date_forms`]）。
//!   表达式模式下缓冲区允许数字与运算符，候选不走拼音解析。
//! - 问字键（缺省 `u`）后跟十六进制码点出那个字符：`u4e00` → 一，`u+1f600` → 😀。模式键本身在 `engine::ModeKeys`。
//! - `vuuid`（全拼下问字键是 `u` 时直接 `uuid`）出 UUID，`vts` 出时间戳（[`generated_forms`]）；值随机或随时间变，
//!   由 Engine 按组句缓存，不走 [`candidates`]。

mod calendar;
mod evaluator;
mod numeral;
mod uuid_forms;

use jiff::Zoned;

use crate::candidate::{Candidate, CandidateKind};

pub use calendar::{
    TIMESTAMP_KEYWORD, date_forms, input_date_forms, time_forms, timestamp_forms, weekday_forms,
};
pub use evaluator::evaluate;
pub use numeral::{
    amount_lower, amount_upper, chinese_decimal_lower, chinese_decimal_upper, chinese_lower,
    chinese_upper,
};
pub use uuid_forms::{UUID_KEYWORD, uuid_forms};

/// 表达式键之后整段是 `uuid` / `ts` 时生成的写法；别的词为 `None`。每次调用都重新生成，组句里的缓存在 Engine。
pub fn generated_forms(keyword: &str, now: &Zoned) -> Option<Vec<String>> {
    match keyword {
        UUID_KEYWORD => Some(uuid_forms()),
        TIMESTAMP_KEYWORD => Some(timestamp_forms(now)),
        _ => None,
    }
}

/// 是不是 [`generated_forms`] 认的词。
pub fn is_generated_keyword(keyword: &str) -> bool {
    matches!(keyword, UUID_KEYWORD | TIMESTAMP_KEYWORD)
}

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

/// 屏幕键盘上的 `c` 在算式里当什么：乘除号（`×` `÷`）换成 `*` `/`，其余算式字符原样，不是算式字符为 `None`。
pub fn expression_char(c: char) -> Option<char> {
    let c = match c {
        '×' => '*',
        '÷' => '/',
        c => c,
    };
    is_expression_char(c).then_some(c)
}

/// 按输入算快捷候选；不是快捷输入时为空。`expression` 是表达式键；`now` 由调用方给，测试可固定时间。
pub fn candidates(input: &str, expression: char, now: &Zoned) -> Vec<Candidate> {
    let texts: Vec<String> = match input {
        "rq" => date_forms(now),
        "sj" => time_forms(now),
        "xq" => weekday_forms(now),
        _ => match input.strip_prefix(expression) {
            Some(body) => expression_forms(body, now),
            None => Vec::new(),
        },
    };
    as_candidates(texts)
}

/// 快捷写法包成候选。
pub(crate) fn as_candidates(texts: Vec<String>) -> Vec<Candidate> {
    texts
        .into_iter()
        .map(|text| Candidate {
            text,
            kind: CandidateKind::Shortcut,
            syllables: Vec::new(),
            reading: None,
            translation: None,
            aux_code: None,
        })
        .collect()
}

/// 打全拼出的词是「日期 / 时间 / 星期」时对应的快捷写法，同 `rq` / `sj` / `xq`；别的词为空。
pub fn keyword_forms(word: &str, now: &Zoned) -> Vec<String> {
    match word {
        "日期" => date_forms(now),
        "时间" => time_forms(now),
        "星期" => weekday_forms(now),
        _ => Vec::new(),
    }
}

/// 金额最多几位小数（角、分）。
const AMOUNT_FRACTION_DIGITS: usize = 2;

/// 表达式键之后的部分：一个数出中文数字与金额，四位年份加月日的日期出日期写法，其余当四则运算出结果与「算式=结果」。
/// 顺序是要紧的：`1998` 与 `1998.3` 是数字与小数，先于日期；四位年份的 `1998-3-8` 才当日期，落在算式之前。
fn expression_forms(body: &str, now: &Zoned) -> Vec<String> {
    let mut forms = number_forms(body).unwrap_or_default();
    // 日期与数字可以并存：`v1234.5` 既是一千二百三十四点五，也可以是 1234 年 5 月，
    // 后者排在后面，不抢前面的常用读法
    let mut dated = false;
    if let Some(date_forms) = input_date_forms(body, now) {
        forms.extend(date_forms);
        dated = true;
    }
    if !forms.is_empty() {
        // 写法同时像日期又像算式时（`v1998-3-8`：1998 年 3 月 8 日 / 1987），两头都给：
        // 日期在前，算式跟在后 —— 别因为多认了日期就把算式的老用法弄丢。
        if dated
            && let Some(result) = evaluate(body)
            && result != body.trim()
        {
            forms.push(result.clone());
            forms.push(format!("{body}={result}"));
        }
        return forms;
    }
    match evaluate(body) {
        Some(result) => vec![result.clone(), format!("{body}={result}")],
        None => Vec::new(),
    }
}

/// 单个非负数（`123`、`123.5`）：中文数字在前，金额在后；小数超过两位没有金额写法。
/// 末尾刚敲了 `.`（`123.`）按整数给，候选不闪。
fn number_forms(body: &str) -> Option<Vec<String>> {
    let (integer, fraction) = body.split_once('.').unwrap_or((body, ""));
    let digits = |text: &str| text.bytes().all(|b| b.is_ascii_digit());
    if integer.is_empty() || !digits(integer) || !digits(fraction) {
        return None;
    }
    let mut forms = if fraction.is_empty() {
        vec![chinese_lower(integer), chinese_upper(integer)]
    } else {
        vec![
            chinese_decimal_lower(integer, fraction),
            chinese_decimal_upper(integer, fraction),
        ]
    };
    if fraction.len() <= AMOUNT_FRACTION_DIGITS {
        forms.push(amount_lower(integer, fraction));
        forms.push(amount_upper(integer, fraction));
    }
    Some(forms)
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
        // 前三条是原有顺序，不动；后面是补的各种写法
        let rq = texts("rq");
        assert_eq!(&rq[..3], ["2026年9月3日", "2026-09-03", "2026/09/03"]);
        assert!(rq.contains(&"2026年9月3日 星期四".to_owned()), "{rq:?}");
        assert!(rq.contains(&"20260903".to_owned()), "{rq:?}");
        assert!(rq.contains(&"二〇二六年九月三日".to_owned()), "{rq:?}");
        let sj = texts("sj");
        assert_eq!(&sj[..3], ["19:06", "19:06:23", "19点06分"]);
        assert!(sj.contains(&"19:06:23.000".to_owned()), "{sj:?}");
        assert!(sj.contains(&"20260903190623".to_owned()), "{sj:?}");
        // Unix 秒 / 毫秒不混在 `sj` 里，要时间戳敲 `vts`
        assert!(
            !sj.iter()
                .any(|t| matches!(t.len(), 10 | 13) && t.bytes().all(|b| b.is_ascii_digit())),
            "{sj:?}"
        );
        let xq = texts("xq");
        assert_eq!(&xq[..2], ["星期四", "周四"]);
        assert!(texts("rqi").is_empty());
    }

    #[test]
    fn timestamp_forms_cover_unix_and_rfc3339() {
        assert_eq!(
            timestamp_forms(&now()),
            [
                "1788462383",
                "2026-09-03T19:06:23.000Z",
                "1788462383000",
                "2026-09-03T19:06:23.000+00:00"
            ]
        );
        let beijing = now().with_time_zone(TimeZone::fixed(jiff::tz::offset(8)));
        assert_eq!(timestamp_forms(&beijing)[1], "2026-09-03T19:06:23.000Z");
        assert_eq!(
            timestamp_forms(&beijing)[3],
            "2026-09-04T03:06:23.000+08:00"
        );
        assert!(is_generated_keyword("ts") && is_generated_keyword("uuid"));
        assert!(!is_generated_keyword("t"));
    }

    #[test]
    fn keyword_shortcuts() {
        assert_eq!(keyword_forms("日期", &now()), texts("rq"));
        assert_eq!(keyword_forms("时间", &now()), texts("sj"));
        assert_eq!(keyword_forms("星期", &now()), texts("xq"));
        assert!(keyword_forms("日起", &now()).is_empty());
    }

    #[test]
    fn expression_shortcuts() {
        assert_eq!(texts("v1+2"), ["3", "1+2=3"]);
        assert_eq!(
            texts("v123"),
            [
                "一百二十三",
                "壹佰贰拾叁",
                "一百二十三元整",
                "壹佰贰拾叁元整"
            ]
        );
        assert_eq!(
            texts("v123.5"),
            [
                "一百二十三点五",
                "壹佰贰拾叁点伍",
                "一百二十三元五角",
                "壹佰贰拾叁元伍角"
            ]
        );
        assert_eq!(texts("v123."), texts("v123"));
        // 超过角分的小数只有读法；带运算符的照常算
        assert_eq!(texts("v3.14159"), ["三点一四一五九", "叁点壹肆壹伍玖"]);
        assert_eq!(texts("v1.5*2"), ["3", "1.5*2=3"]);
        assert!(texts("v").is_empty());
        assert!(texts("v1+").is_empty());
        assert!(texts("very").is_empty());
    }

    #[test]
    fn date_inputs_in_expression_mode() {
        let date = texts("v1998.3.8");
        assert_eq!(date[0], "一九九八年三月八日");
        assert_eq!(date[1], "1998年3月8日");
        assert_eq!(date[2], "1998-03-08");
        // `-` 与 `/` 也是分隔符：日期在前，算式结果跟在后面，两者都保留
        let dashed = texts("v1998-3-8");
        assert_eq!(dashed[0], "一九九八年三月八日");
        assert!(dashed.iter().any(|t| t == "1987"), "{dashed:?}");
        let slashed = texts("v1998/3/8");
        assert_eq!(slashed[0], "一九九八年三月八日");
        assert!(slashed.iter().any(|t| t == "83.25"), "{slashed:?}");
        // 补零也认
        assert_eq!(texts("v1998.03.08")[0], "一九九八年三月八日");
        assert_eq!(texts("v2006-12-25")[0], "二〇〇六年十二月二十五日");
        // 混用分隔符不算日期（`1998-3.8` 切成什么说不清），只剩算式
        assert_eq!(texts("v1998-3.8")[0], "1994.2");
        // 年份不够、段数不对、月日不合法 → 仍按算式
        assert_eq!(texts("v1-2-3"), ["-4", "1-2-3=-4"]);
        assert_eq!(texts("v98-3-8"), ["87", "98-3-8=87"]);
        assert_eq!(texts("v1998-2-30"), ["1966", "1998-2-30=1966"]);
        assert_eq!(texts("v1998-3-8-1"), ["1986", "1998-3-8-1=1986"]);
        // 数字与小数写法不受日期影响
        assert_eq!(texts("v1998")[0], "一千九百九十八");
        assert_eq!(texts("v1998.3")[0], "一千九百九十八点三");
        // 数字之后还会跟一条日期读法，不抢第一位
        assert!(texts("v1998.3").iter().any(|t| t.contains("年")));
    }

    #[test]
    fn date_precision_follows_segment_count() {
        // 一段＝年、两段＝年月、三段＝年月日，三种分隔符一致
        for sep in ['.', '-', '/'] {
            assert!(
                texts("v1632").iter().any(|t| t.contains("年")),
                "v1632 应给日期读法"
            );
            let year_month = format!("v1632{sep}5");
            assert!(
                texts(&year_month).iter().any(|t| t == "1632年5月"),
                "{year_month} 应给年月"
            );
            let full = format!("v1632{sep}5{sep}20");
            assert!(
                texts(&full).iter().any(|t| t == "1632-05-20"),
                "{full} 应给完整日期"
            );
        }
    }

    #[test]
    fn date_shift_by_days() {
        // 往前 7 天、往后 30 天
        let back = texts("v1234.9.9-7");
        assert!(back.iter().any(|t| t == "1234-09-02"), "{back:?}");
        assert_eq!(back[0], "一二三四年九月二日", "{back:?}");
        let forward = texts("v1234.9.9+30");
        assert!(forward.iter().any(|t| t == "1234-10-09"), "{forward:?}");
        // 跨月、跨年、闰年都按日历算
        assert!(texts("v2024.2.28+1").iter().any(|t| t == "2024-02-29"));
        assert!(texts("v2023.2.28+1").iter().any(|t| t == "2023-03-01"));
        assert!(texts("v2023.12.31+1").iter().any(|t| t == "2024-01-01"));
        assert!(texts("v2024.1.1-1").iter().any(|t| t == "2023-12-31"));
        // 偏移的减号不能和日期的减号撞（那是四段日期）
        assert!(texts("v1234/9/9+7").iter().any(|t| t == "1234-09-16"));
        assert!(texts("v1234-9-9+7").iter().any(|t| t == "1234-09-16"));
        assert_eq!(texts("v1998-3-8-1"), ["1986", "1998-3-8-1=1986"]);
        assert_eq!(texts("v1-2-3"), ["-4", "1-2-3=-4"]);
        assert_eq!(texts("v98-3-8")[0], "87");
    }

    #[test]
    fn now_and_offsets() {
        // 测试把「现在」固定在 2026-09-03（见 tests::now）
        let today = texts("vnow");
        assert_eq!(today[0], "二〇二六年九月三日", "{today:?}");
        assert!(today.iter().any(|t| t == "2026-09-03"), "{today:?}");
        assert!(texts("vnow+4").iter().any(|t| t == "2026-09-07"));
        assert!(texts("vnow-4").iter().any(|t| t == "2026-08-30"));
        // 分号 / 冒号分隔，符号可省
        assert!(texts("vnow;-4").iter().any(|t| t == "2026-08-30"));
        assert!(texts("vnow;4").iter().any(|t| t == "2026-09-07"));
        assert!(texts("vnow：+10").iter().any(|t| t == "2026-09-13"));
        // `now` 之外的写法不变
        assert!(texts("v2026.9.3").iter().any(|t| t == "2026-09-03"));
        assert_eq!(texts("v1+2")[0], "3");
    }

    #[test]
    fn expression_chars() {
        assert!(is_expression_char('7'));
        assert!(is_expression_char('('));
        assert!(!is_expression_char('='));
        assert!(!is_expression_char('x'));
        assert_eq!(expression_char('×'), Some('*'));
        assert_eq!(expression_char('÷'), Some('/'));
        assert_eq!(expression_char('3'), Some('3'));
        assert_eq!(expression_char('='), None);
    }
}
