//! 自定义短语配置及共享校验、预览；由 Core 匹配，平台负责编辑与展示。

use std::borrow::Cow;
use std::ops::RangeInclusive;

use jiff::Zoned;
use jiff::civil::Weekday;

use serde::{Deserialize, Serialize};

use crate::i18n::{tr, tr_args};

/// 输入码最长几个字母。
pub const MAX_CODE_LEN: usize = 32;

/// 新建短语的权重。
pub const DEFAULT_WEIGHT: u32 = 10;

/// 权重的取值范围。
pub const WEIGHTS: RangeInclusive<u32> = 1..=999;

/// 按原始输入码匹配的文本；同一个输入码下按权重从高到低排在普通候选前面。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "StoredPhrase")]
pub struct CustomPhrase {
    /// 小写英文字母输入码，1–32 个字符。
    pub code: String,

    /// 原样上屏的文本，保留空格和换行。
    pub text: String,

    /// 越大越靠前，可以相同；相同的按列表顺序。
    pub weight: u32,

    /// 是否启用。
    pub enabled: bool,
}

/// 配置里的一条：0.3 之前写的是固定位置 `position`（1–9），读进来换成权重，第 1 位最高。
#[derive(Deserialize)]
struct StoredPhrase {
    code: String,
    text: String,
    weight: Option<u32>,
    position: Option<u32>,
    #[serde(default = "enabled")]
    enabled: bool,
}

impl TryFrom<StoredPhrase> for CustomPhrase {
    type Error = String;

    fn try_from(stored: StoredPhrase) -> Result<Self, String> {
        let weight = match (stored.weight, stored.position) {
            (Some(weight), _) => weight,
            (None, Some(position @ 1..=9)) => DEFAULT_WEIGHT + 1 - position,
            (None, Some(_)) => return Err(tr("phrases.position").into()),
            (None, None) => DEFAULT_WEIGHT,
        };
        Ok(Self {
            code: stored.code,
            text: stored.text,
            weight,
            enabled: stored.enabled,
        })
    }
}

fn enabled() -> bool {
    true
}

/// 短语里的时间变量（#800），出候选时按当时的本地时间换掉，候选窗里显示的就是要上屏的：
/// `{year}` 四位年；`{month}` `{day}` `{hour}` 不补零；`{minute}` `{second}` 补到两位（读成「9 时 05 分」）；
/// `{weekday}` 是「一」到「日」（写成「星期{weekday}」）。数字变量后面加 `:0` 和位数补零（`{month:02}` 出 03，#906），
/// 写法同 Rust 的 `format!`。`{{` `}}` 输出花括号本身，认不出的 `{…}` 原样留着。
pub fn expand_variables<'a>(text: &'a str, now: &Zoned) -> Cow<'a, str> {
    if !text.contains(['{', '}']) {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find(['{', '}']) {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        if let Some(after) = tail.strip_prefix("{{") {
            out.push('{');
            rest = after;
            continue;
        }
        if let Some(after) = tail.strip_prefix("}}") {
            out.push('}');
            rest = after;
            continue;
        }
        let value = tail
            .strip_prefix('{')
            .and_then(|body| body.split_once('}'))
            .and_then(|(name, after)| Some((variable(name, now)?, after)));
        match value {
            Some((value, after)) => {
                out.push_str(&value);
                rest = after;
            }
            None => {
                out.push_str(&tail[..1]);
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    Cow::Owned(out)
}

fn variable(name: &str, now: &Zoned) -> Option<String> {
    if let Some((name, spec)) = name.split_once(':') {
        let width = spec.strip_prefix('0')?.parse::<usize>().ok()?;
        let value = variable(name, now)?;
        if name == "weekday" || !(1..=9).contains(&width) {
            return None;
        }
        return Some(format!("{value:0>width$}"));
    }
    Some(match name {
        "year" => now.year().to_string(),
        "month" => now.month().to_string(),
        "day" => now.day().to_string(),
        "hour" => now.hour().to_string(),
        "minute" => format!("{:02}", now.minute()),
        "second" => format!("{:02}", now.second()),
        "weekday" => match now.weekday() {
            Weekday::Monday => "一",
            Weekday::Tuesday => "二",
            Weekday::Wednesday => "三",
            Weekday::Thursday => "四",
            Weekday::Friday => "五",
            Weekday::Saturday => "六",
            Weekday::Sunday => "日",
        }
        .to_owned(),
        _ => return None,
    })
}

/// 设置界面里填的权重：去掉首尾空白后须是 [`WEIGHTS`] 里的整数。
pub fn parse_weight(input: &str) -> Result<u32, String> {
    input
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|weight| WEIGHTS.contains(weight))
        .ok_or_else(weight_error)
}

fn weight_error() -> String {
    tr_args(
        "phrases.weight",
        &[("min", WEIGHTS.start()), ("max", WEIGHTS.end())],
    )
}

/// 保存和加载使用同一校验，不覆盖冲突条目。
pub fn validate_phrases(phrases: &[CustomPhrase]) -> Result<(), String> {
    let mut seen = std::collections::BTreeSet::new();
    for phrase in phrases {
        if !valid_code(&phrase.code) {
            return Err(tr_args("phrases.code", &[("max", &MAX_CODE_LEN)]));
        }
        if !WEIGHTS.contains(&phrase.weight) {
            return Err(weight_error());
        }
        if phrase.text.is_empty() {
            return Err(tr("phrases.empty").into());
        }
        if !seen.insert((&phrase.code, &phrase.text)) {
            return Err(tr_args("phrases.duplicate", &[("code", &phrase.code)]));
        }
    }
    Ok(())
}

/// 输入码能不能当自定义短语用：1–[`MAX_CODE_LEN`] 个小写英文字母。
pub fn valid_code(code: &str) -> bool {
    !code.is_empty() && code.len() <= MAX_CODE_LEN && code.bytes().all(|c| c.is_ascii_lowercase())
}

/// 把系统的文本替换（macOS「键盘 → 文本替换」这类「输入码 → 短语」表）并进自定义短语：
/// 每条用缺省权重接在后面，同权重时排在用户自己的短语之后；输入码不是小写字母、短语为空、
/// 或用户已有同码同文本的规则时跳过。结果保证通过 [`validate_phrases`]（前提是 `base` 本身合法）。
pub fn merge_replacements<'a>(
    base: &[CustomPhrase],
    replacements: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Vec<CustomPhrase> {
    let mut phrases = base.to_vec();
    for (code, text) in replacements {
        if !valid_code(code) || text.is_empty() {
            continue;
        }
        if phrases.iter().any(|p| p.code == code && p.text == text) {
            continue;
        }
        phrases.push(CustomPhrase::new(code, text));
    }
    phrases
}

/// 导入的短语并进现有列表：原样接在后面（保留权重与开关），已有同码同文本的跳过。返回合并结果与新加了几条。
pub fn merge_imported(
    base: &[CustomPhrase],
    imported: impl IntoIterator<Item = CustomPhrase>,
) -> (Vec<CustomPhrase>, usize) {
    let mut phrases = base.to_vec();
    let mut added = 0;
    for phrase in imported {
        if phrases
            .iter()
            .any(|p| p.code == phrase.code && p.text == phrase.text)
        {
            continue;
        }
        phrases.push(phrase);
        added += 1;
    }
    (phrases, added)
}

/// 候选菜单里的「置顶」：让 `text` 成为输入码 `code` 下排第一的短语。权重取这个码别的短语里最高的加一
/// （封顶 999，没有别的就用缺省权重），并挪到同码短语的最前面，权重打平也是它先。已有同码同文本的就改那条。
/// 已经是唯一最高的返回 `false`，列表不动。
pub fn pin(phrases: &mut Vec<CustomPhrase>, code: &str, text: &str) -> bool {
    let top = phrases
        .iter()
        .filter(|p| p.enabled && p.code == code && p.text != text)
        .map(|p| p.weight)
        .max();
    let existing = phrases
        .iter()
        .position(|p| p.code == code && p.text == text);
    let mut phrase = match existing {
        Some(index) => {
            let phrase = &phrases[index];
            if phrase.enabled && top.is_none_or(|top| phrase.weight > top) {
                return false;
            }
            phrases.remove(index)
        }
        None => CustomPhrase::new(code, text),
    };
    if let Some(top) = top {
        phrase.weight = (top + 1).min(*WEIGHTS.end());
    }
    phrase.enabled = true;
    let first = phrases.iter().position(|p| p.code == code);
    phrases.insert(first.unwrap_or(phrases.len()), phrase);
    true
}

/// 候选菜单里的「取消置顶」：删掉输入码 `code` 下文字是 `text` 的短语。返回有没有删。
pub fn unpin(phrases: &mut Vec<CustomPhrase>, code: &str, text: &str) -> bool {
    let before = phrases.len();
    phrases.retain(|p| !(p.code == code && p.text == text));
    phrases.len() < before
}

impl CustomPhrase {
    /// 缺省权重、启用的一条。
    pub fn new(code: &str, text: &str) -> Self {
        Self {
            code: code.to_owned(),
            text: text.to_owned(),
            weight: DEFAULT_WEIGHT,
            enabled: true,
        }
    }

    /// 单行预览保留 Unicode 字符边界，用可见符号表示换行和制表符。
    pub fn preview(text: &str, max_chars: usize) -> String {
        let mut chars = text.chars().peekable();
        let mut preview = String::new();
        for _ in 0..max_chars {
            let Some(c) = chars.next() else {
                break;
            };
            preview.push(match c {
                '\r' => {
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    '↵'
                }
                '\n' => '↵',
                '\t' => '⇥',
                _ => c,
            });
        }
        if chars.next().is_some() {
            preview.push('…');
        }
        preview
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CustomPhrase, DEFAULT_WEIGHT, expand_variables, merge_imported, merge_replacements,
        parse_weight, pin, unpin, validate_phrases,
    };

    #[test]
    fn time_variables_expand_and_braces_escape() {
        // 固定偏移不查时区库：CI 的 Linux 容器里没有 tzdata，按 `Asia/Shanghai` 解析会失败
        let now = jiff::civil::date(2026, 3, 8)
            .at(9, 5, 7, 0)
            .to_zoned(jiff::tz::TimeZone::fixed(jiff::tz::offset(8)))
            .unwrap();
        assert_eq!(
            expand_variables(
                "今天是{year}年{month}月{day}日 {hour}:{minute}:{second}，星期{weekday}",
                &now
            ),
            "今天是2026年3月8日 9:05:07，星期日"
        );
        assert_eq!(
            expand_variables("{year}-{month:02}-{day:02} {hour:02}:{minute:02}", &now),
            "2026-03-08 09:05"
        );
        // 补零只认 `:0` 加 1–9 位，星期不是数字；认不出的原样留着
        assert_eq!(
            expand_variables("{month:2} {day:00} {weekday:02} {year:05}", &now),
            "{month:2} {day:00} {weekday:02} 02026"
        );
        assert_eq!(
            expand_variables("{{year}} {unknown} }", &now),
            "{year} {unknown} }"
        );
        assert!(matches!(
            expand_variables("没有变量", &now),
            std::borrow::Cow::Borrowed(_)
        ));
    }

    #[test]
    fn imports_keep_weights_and_skip_what_is_already_there() {
        let base = vec![
            CustomPhrase::new("ee", "："),
            CustomPhrase::new("dz", "地址"),
        ];
        let imported = vec![
            CustomPhrase {
                weight: 50,
                enabled: false,
                ..CustomPhrase::new("ee", "；")
            },
            CustomPhrase {
                weight: 99,
                ..CustomPhrase::new("dz", "地址")
            },
        ];
        let (merged, added) = merge_imported(&base, imported.clone());
        assert_eq!(added, 1);
        assert_eq!(
            merged,
            vec![base[0].clone(), base[1].clone(), imported[0].clone()]
        );
        validate_phrases(&merged).unwrap();
    }

    #[test]
    fn replacements_follow_the_users_phrases() {
        let base = vec![
            CustomPhrase::new("yx", "第一条"),
            CustomPhrase::new("ee", "："),
        ];
        let merged = merge_replacements(
            &base,
            [
                ("yx", "qi@example.com"),
                ("omw", "On my way!"),
                ("ee", "："),
                ("Gs", "大写不算"),
                ("a1", "带数字不算"),
                ("", "空码不算"),
                ("kong", ""),
            ],
        );
        validate_phrases(&merged).unwrap();
        assert_eq!(merged.len(), 4);
        assert_eq!(merged[2], CustomPhrase::new("yx", "qi@example.com"));
        assert_eq!(merged[3], CustomPhrase::new("omw", "On my way!"));
    }

    #[test]
    fn same_weights_are_allowed_but_not_the_same_phrase() {
        let mut phrases = vec![CustomPhrase::new("ee", "："), CustomPhrase::new("ee", "；")];
        validate_phrases(&phrases).unwrap();
        phrases[1].weight = 0;
        assert!(validate_phrases(&phrases).is_err());
        phrases[1] = CustomPhrase::new("ee", "：");
        assert!(validate_phrases(&phrases).unwrap_err().contains("ee"));
    }

    #[test]
    fn typed_weights_must_be_whole_numbers_in_range() {
        assert_eq!(parse_weight(" 25 "), Ok(25));
        for input in ["", "0", "1000", "-1", "2.5", "十"] {
            assert!(parse_weight(input).is_err(), "{input}");
        }
    }

    #[test]
    fn old_positions_become_weights() {
        let read = |fields: &str| {
            toml::from_str::<CustomPhrase>(&format!("code = \"ee\"\ntext = \"：\"\n{fields}"))
        };
        assert_eq!(read("position = 1").unwrap().weight, DEFAULT_WEIGHT);
        assert_eq!(read("position = 9").unwrap().weight, 2);
        assert_eq!(read("position = 3\nweight = 50").unwrap().weight, 50);
        assert_eq!(read("").unwrap().weight, DEFAULT_WEIGHT);
        assert!(read("position = 10").is_err());
        assert!(!read("enabled = false").unwrap().enabled);
    }

    #[test]
    fn pinning_beats_every_other_phrase_of_the_code() {
        let mut phrases = vec![
            CustomPhrase::new("qj", "青简"),
            CustomPhrase {
                weight: 30,
                ..CustomPhrase::new("qj", "前进")
            },
            CustomPhrase::new("cp", "C盘"),
        ];
        assert!(pin(&mut phrases, "qj", "前几"));
        assert_eq!(phrases[0].text, "前几");
        assert_eq!(phrases[0].weight, 31);

        // 已有的短语挪到最前、权重抬上去；已经最高就不动
        assert!(pin(&mut phrases, "qj", "青简"));
        assert_eq!((phrases[0].text.as_str(), phrases[0].weight), ("青简", 32));
        assert!(!pin(&mut phrases, "qj", "青简"));

        // 码下没有别的短语用缺省权重
        assert!(pin(&mut phrases, "sj", "时间"));
        assert_eq!(phrases.last().unwrap().weight, DEFAULT_WEIGHT);
        assert!(validate_phrases(&phrases).is_ok());

        assert!(unpin(&mut phrases, "qj", "前几"));
        assert!(!unpin(&mut phrases, "qj", "前几"));
        assert_eq!(phrases.len(), 4);
    }

    #[test]
    fn pinning_at_the_top_weight_still_comes_first() {
        let mut phrases = vec![CustomPhrase {
            weight: 999,
            ..CustomPhrase::new("qj", "前进")
        }];
        pin(&mut phrases, "qj", "前几");
        assert_eq!(phrases[0].text, "前几");
        assert_eq!(phrases[0].weight, 999);
    }
}
