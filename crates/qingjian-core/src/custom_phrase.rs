//! 自定义短语配置及共享校验、预览；由 Core 匹配，平台负责编辑与展示。

use serde::{Deserialize, Serialize};

/// 按原始输入码匹配的固定位置文本。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomPhrase {
    /// 输入码：1–32 个字符，小写英文字母、数字或汉字。
    pub code: String,

    /// 原样上屏的文本，保留空格和换行。
    pub text: String,

    /// 从 1 开始的固定候选位置，最大 9。
    pub position: usize,

    /// 是否启用；停用规则仍保留位置。
    #[serde(default = "enabled")]
    pub enabled: bool,
}

fn enabled() -> bool {
    true
}

/// 保存和加载使用同一校验，不覆盖冲突条目。
pub fn validate_phrases(phrases: &[CustomPhrase]) -> Result<(), String> {
    let mut occupied = std::collections::BTreeSet::new();
    for phrase in phrases {
        if !valid_code(&phrase.code) {
            return Err("输入码须为 1–32 个小写英文字母、数字或汉字".into());
        }
        if !(1..=9).contains(&phrase.position) {
            return Err("候选位置须为 1–9".into());
        }
        if phrase.text.is_empty() {
            return Err("自定义短语不能为空".into());
        }
        if !occupied.insert((&phrase.code, phrase.position)) {
            return Err(format!(
                "输入码 {} 的第 {} 位已被占用，不能保存",
                phrase.code, phrase.position
            ));
        }
    }
    Ok(())
}

/// 输入码能不能当自定义短语用：1–32 个小写字母、数字或汉字（按 Unicode 字符计）。
pub fn valid_code(code: &str) -> bool {
    let count = code.chars().count();
    (1..=32).contains(&count) && code.chars().all(is_code_char)
}

fn is_code_char(ch: char) -> bool {
    ch.is_ascii_lowercase() || ch.is_ascii_digit() || is_cjk(ch)
}

/// 只把 ASCII 大写收成小写，数字与汉字原样保留。
pub fn normalize_code(code: &str) -> String {
    code.chars()
        .map(|ch| {
            if ch.is_ascii_uppercase() {
                ch.to_ascii_lowercase()
            } else {
                ch
            }
        })
        .collect()
}

/// 把系统的文本替换（macOS「键盘 → 文本替换」这类「输入码 → 短语」表）并进自定义短语：
/// 每条占该输入码最靠前的空位（1–9），输入码不合法、短语为空、九位都满、或用户已有同码同文本的规则时跳过。
/// 结果保证通过 [`validate_phrases`]（前提是 `base` 本身合法）。
pub fn merge_replacements<'a>(
    base: &[CustomPhrase],
    replacements: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Vec<CustomPhrase> {
    let mut phrases = base.to_vec();
    for (code, text) in replacements {
        // 系统替换表只收已经合法的码（含数字 / 汉字）；带大写的条目不算，与原先一致。
        if !valid_code(code) || text.is_empty() {
            continue;
        }
        if phrases.iter().any(|p| p.code == code && p.text == text) {
            continue;
        }
        let Some(position) =
            (1..=9).find(|n| !phrases.iter().any(|p| p.code == code && p.position == *n))
        else {
            continue;
        };
        phrases.push(CustomPhrase {
            code: code.to_owned(),
            text: text.to_owned(),
            position,
            enabled: true,
        });
    }
    phrases
}

impl CustomPhrase {
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

/// 按内容建议输入码：取内容前最多三个可用字符（字母收小写、数字、汉字）；
/// 若前缀推不出（只剩标点），再按 `initial_of` 取汉字拼音首字母。
pub fn suggest_code(text: &str, mut initial_of: impl FnMut(char) -> Option<char>) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        if out.chars().count() >= 3 {
            break;
        }
        if ch.is_ascii_alphabetic() {
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_ascii_digit() || is_cjk(ch) {
            out.push(ch);
        }
    }
    if !out.is_empty() {
        return out;
    }
    for ch in text.chars() {
        if out.chars().count() >= 3 {
            break;
        }
        if let Some(initial) = initial_of(ch).filter(|c| c.is_ascii_alphabetic()) {
            out.push(initial.to_ascii_lowercase());
        }
    }
    out
}

fn is_cjk(ch: char) -> bool {
    matches!(
        ch,
        '\u{4E00}'..='\u{9FFF}' // CJK Unified Ideographs
        | '\u{3400}'..='\u{4DBF}' // Extension A
        | '\u{F900}'..='\u{FAFF}' // Compatibility Ideographs
    )
}

#[cfg(test)]
mod tests {
    use super::{
        CustomPhrase, merge_replacements, normalize_code, suggest_code, valid_code, validate_phrases,
    };

    fn phrase(code: &str, text: &str, position: usize) -> CustomPhrase {
        CustomPhrase {
            code: code.into(),
            text: text.into(),
            position,
            enabled: true,
        }
    }

    #[test]
    fn suggest_code_takes_first_three_letters_digits_or_han() {
        let initials = |ch: char| match ch {
            '你' => Some('n'),
            '好' => Some('h'),
            _ => None,
        };
        assert_eq!(suggest_code("你好世界", initials), "你好世");
        assert_eq!(suggest_code("13800138000", |_| None), "138");
        assert_eq!(suggest_code("Bearer token", |_| None), "bea");
        assert_eq!(suggest_code("b", |_| None), "b");
        assert_eq!(suggest_code("，，，", |_| None), "");
    }

    #[test]
    fn codes_allow_digits_and_han() {
        assert!(valid_code("138"));
        assert!(valid_code("你好世"));
        assert!(valid_code("a1"));
        assert!(!valid_code("Gs"));
        assert!(!valid_code(""));
        assert_eq!(normalize_code("AbC138你好"), "abc138你好");
        validate_phrases(&[phrase("138", "13800138000", 1)]).unwrap();
        validate_phrases(&[phrase("你好", "Hello", 1)]).unwrap();
    }

    #[test]
    fn replacements_take_the_first_free_slot_of_their_code() {
        let base = vec![phrase("yx", "第一位", 1), phrase("ee", "：", 1)];
        let merged = merge_replacements(
            &base,
            [
                ("yx", "qi@example.com"),
                ("omw", "On my way!"),
                ("ee", "："),
                ("Gs", "大写不算"),
                ("", "空码不算"),
                ("kong", ""),
            ],
        );
        validate_phrases(&merged).unwrap();
        assert_eq!(merged.len(), 4);
        assert_eq!(merged[2], phrase("yx", "qi@example.com", 2));
        assert_eq!(merged[3], phrase("omw", "On my way!", 1));
    }

    #[test]
    fn replacements_stop_at_nine_slots_per_code() {
        let base: Vec<_> = (1..=9).map(|n| phrase("x", &n.to_string(), n)).collect();
        let merged = merge_replacements(&base, [("x", "第十条")]);
        assert_eq!(merged.len(), 9);
    }
}
