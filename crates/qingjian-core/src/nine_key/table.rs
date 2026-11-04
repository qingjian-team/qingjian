//! 数字码到拼音写法的查表，从全拼音节表生成。

use std::collections::HashMap;
use std::sync::LazyLock;

use super::{INTERJECTIONS, Position, digit_of, interjection};
use crate::parser::{INITIALS, MAX_SYLLABLE_LEN, SYLLABLES};

pub(super) static CODE_TABLE: LazyLock<CodeTable> = LazyLock::new(CodeTable::build);

/// 数字码 → 写法的三张表，从全拼音节表生成。
pub(super) struct CodeTable {
    /// 码 → 完整音节。
    syllables: HashMap<String, Vec<&'static str>>,

    /// 码 → 音节或音节的前缀（末尾位置用）。
    prefixes: HashMap<String, Vec<String>>,

    /// 码 → 声母（简拼位置用）。
    initials: HashMap<String, Vec<&'static str>>,
}

impl CodeTable {
    fn build() -> Self {
        let mut syllables: HashMap<String, Vec<&'static str>> = HashMap::new();
        let mut prefixes: HashMap<String, Vec<String>> = HashMap::new();
        for syllable in SYLLABLES {
            syllables.entry(code(syllable)).or_default().push(syllable);
            for end in 1..=syllable.len() {
                let prefix = &syllable[..end];
                let entry = prefixes.entry(code(prefix)).or_default();
                if !entry.iter().any(|known| known == prefix) {
                    entry.push(prefix.to_owned());
                }
            }
        }
        let mut initials: HashMap<String, Vec<&'static str>> = HashMap::new();
        for initial in INITIALS {
            initials.entry(code(initial)).or_default().push(initial);
        }
        Self {
            syllables,
            prefixes,
            initials,
        }
    }

    /// `span`（数字与已选定字母）能不能当一个位置；`last` 表示它一直到输入末尾，`pinned_here` 表示它从开头或 `'` 后起。
    /// 完整音节优先；末尾按前缀读（打完的音节也算前缀）；中间只有声母能当简拼。
    pub(super) fn position(
        &self,
        span: &str,
        start: usize,
        last: bool,
        pinned_here: bool,
    ) -> Option<Position> {
        // 拼音列选定的叹词读法（ng / hm / hng）自成一个位置；数字不往这儿切，免得 64 多一种切法
        if pinned_here && span.len() > 1 && interjection(span).is_some() {
            return Some(Position {
                start,
                len: span.len(),
                alternatives: vec![span.to_owned()],
                complete: true,
                prefix: false,
            });
        }
        let code: String = span.chars().map(|c| digit_of(c).unwrap_or(c)).collect();
        let fits = |text: &str| {
            span.bytes()
                .zip(text.bytes())
                .all(|(typed, letter)| typed.is_ascii_digit() || typed == letter)
        };
        let complete: Vec<&str> = self
            .syllables
            .get(&code)
            .into_iter()
            .flatten()
            .copied()
            .filter(|text| fits(text))
            .collect();
        let alternatives: Vec<String> = if last {
            self.prefixes
                .get(&code)
                .into_iter()
                .flatten()
                .filter(|text| fits(text))
                .cloned()
                .collect()
        } else if !complete.is_empty() {
            complete.iter().map(|text| (*text).to_owned()).collect()
        } else {
            self.initials
                .get(&code)
                .into_iter()
                .flatten()
                .filter(|text| fits(text))
                .map(|text| (*text).to_owned())
                .collect()
        };
        if alternatives.is_empty() {
            return None;
        }
        Some(Position {
            start,
            len: span.len(),
            alternatives,
            complete: !complete.is_empty(),
            prefix: last || complete.is_empty(),
        })
    }

    /// 从 `start` 起各种长度能读成的完整音节与叹词读法，外加一码的首字母；遇到 `'` 停。长的在前。
    pub(super) fn spellings_from(&self, input: &str, start: usize) -> Vec<String> {
        let mut found: Vec<String> = Vec::new();
        let limit = (start + MAX_SYLLABLE_LEN).min(input.len());
        for end in (start + 1..=limit).rev() {
            let span = &input[start..end];
            if span.contains('\'') {
                continue;
            }
            let code: String = span.chars().map(|c| digit_of(c).unwrap_or(c)).collect();
            let fits = |text: &str| {
                span.bytes()
                    .zip(text.bytes())
                    .all(|(typed, letter)| typed.is_ascii_digit() || typed == letter)
            };
            let mut push = |text: &str| {
                if fits(text) && !found.iter().any(|known| known == text) {
                    found.push(text.to_owned());
                }
            };
            for text in self.syllables.get(&code).into_iter().flatten() {
                push(text);
            }
            for (text, _) in INTERJECTIONS {
                if text.len() > 1 && code == self::code(text) {
                    push(text);
                }
            }
            if end == start + 1 {
                for text in self.prefixes.get(&code).into_iter().flatten() {
                    push(text);
                }
            }
        }
        found
    }
}

fn code(letters: &str) -> String {
    letters.chars().filter_map(digit_of).collect()
}
