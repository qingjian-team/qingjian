//! 自定义短语配置及共享校验、预览；由 Core 匹配，平台负责编辑与展示。

use serde::{Deserialize, Serialize};

/// 按原始输入码匹配的固定位置文本。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomPhrase {
    /// 小写英文字母输入码，1–32 个字符。
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

/// 整表校验：任一条不合格就整体拒绝。**写入**路径用它（保存的短语要么全对要么不落盘）。
/// 读配置时一条坏了不该连累其他条，那时按条调 [`validate_phrase`]。
pub fn validate_phrases(phrases: &[CustomPhrase]) -> Result<(), String> {
    let mut occupied = std::collections::BTreeSet::new();
    for phrase in phrases {
        validate_phrase(phrase, &mut occupied)?;
    }
    Ok(())
}

/// 单条校验，`occupied` 是此前各条已占的「输入码 + 位置」。
///
/// 写入路径要的是全表拒绝（[`validate_phrases`]）；读配置时一条坏了不该连累其他条，
/// 那时按条调这个，把不合格的挑出来丢掉（见 [`retain_valid`]）。
pub fn validate_phrase(
    phrase: &CustomPhrase,
    occupied: &mut std::collections::BTreeSet<(String, usize)>,
) -> Result<(), String> {
    if !valid_code(&phrase.code) {
        return Err("输入码须为 1–32 个小写英文字母".into());
    }
    if !(1..=9).contains(&phrase.position) {
        return Err("候选位置须为 1–9".into());
    }
    if phrase.text.is_empty() {
        return Err("自定义短语不能为空".into());
    }
    if !occupied.insert((phrase.code.clone(), phrase.position)) {
        return Err(format!(
            "输入码 {} 的第 {} 位已被占用，不能保存",
            phrase.code, phrase.position
        ));
    }
    Ok(())
}

/// 输入码能不能当自定义短语用：1–32 个小写英文字母。
fn valid_code(code: &str) -> bool {
    !code.is_empty() && code.len() <= 32 && code.bytes().all(|c| c.is_ascii_lowercase())
}

/// 把系统的文本替换（macOS「键盘 → 文本替换」这类「输入码 → 短语」表）并进自定义短语：
/// 每条占该输入码最靠前的空位（1–9），输入码不是小写字母、短语为空、九位都满、或用户已有同码同文本的规则时跳过。
/// 结果保证通过 [`validate_phrases`]（前提是 `base` 本身合法）。
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

#[cfg(test)]
mod tests {
    use super::{CustomPhrase, merge_replacements, validate_phrases};

    fn phrase(code: &str, text: &str, position: usize) -> CustomPhrase {
        CustomPhrase {
            code: code.into(),
            text: text.into(),
            position,
            enabled: true,
        }
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
                ("a1", "带数字不算"),
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
