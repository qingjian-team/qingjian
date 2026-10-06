//! 特殊输入候选：日期、农历、UUID、符号、拆字、码点与算式。

use super::{calculator, calendar, money, profile::Profile};
use crate::candidate::{Candidate, CandidateKind};
use crate::shortcut;

pub(super) fn candidate(text: String, reading: Option<String>) -> Candidate {
    Candidate {
        text,
        kind: CandidateKind::Shortcut,
        syllables: Vec::new(),
        reading,
        translation: None,
        aux_code: None,
    }
}

pub(super) fn accepts(input: &str, c: char) -> bool {
    if input.starts_with("cC") {
        return c.is_ascii_digit()
            || matches!(
                c,
                '+' | '-' | '*' | '/' | '(' | ')' | '.' | '^' | '%' | ',' | '!'
            );
    }
    if input
        .strip_prefix('R')
        .is_some_and(|body| body.bytes().all(|b| b.is_ascii_digit() || b == b'.'))
    {
        return c.is_ascii_digit() || c == '.';
    }
    if input.starts_with('N')
        || input
            .strip_prefix('U')
            .is_some_and(|body| body.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return c.is_ascii_digit();
    }
    if input == "v" {
        return c.is_ascii_digit();
    }
    if input == "v1" {
        return c == '0';
    }
    c == '`' && !input.is_empty()
}

impl Profile {
    pub(super) fn special(&self, input: &str, now: &jiff::Zoned) -> Option<Vec<Candidate>> {
        let texts = if let Some(kind) = self.date_keys.get(input) {
            calendar::forms(kind, now)
        } else if input == self.uuid_key {
            let mut cache = self.uuid_cache.borrow_mut();
            vec![
                cache
                    .get_or_insert_with(|| uuid::Uuid::new_v4().to_string())
                    .clone(),
            ]
        } else if input == self.lunar_key {
            let (text, comment) = self.lunar(now.date());
            return Some(vec![candidate(
                text,
                (!comment.is_empty()).then_some(comment),
            )]);
        } else if let Some(body) = input.strip_prefix('N') {
            if !body.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            let (text, comment) = if body.len() != 8 {
                ("输入完整的日期".to_owned(), "YYYYMMDD".to_owned())
            } else {
                match format!("{}-{}-{}", &body[..4], &body[4..6], &body[6..])
                    .parse::<jiff::civil::Date>()
                {
                    Ok(date) => self.lunar(date),
                    Err(_) => ("错误".to_owned(), "无效的公历日期".to_owned()),
                }
            };
            return Some(vec![candidate(text, Some(comment))]);
        } else if let Some(body) = input.strip_prefix('U') {
            if !body.bytes().all(|b| b.is_ascii_hexdigit()) {
                return None;
            }
            let values = shortcut::unicode_form(&format!("+{body}"));
            values.into_iter().collect()
        } else if let Some(body) = input.strip_prefix('R') {
            if !body.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
                return None;
            }
            return Some(money::forms(body));
        } else if let Some(body) = input.strip_prefix("cC") {
            if body.is_empty() {
                return None;
            }
            return Some(match calculator::evaluate(body) {
                Some(value) => vec![
                    candidate(value.clone(), None),
                    candidate(format!("{body}={value}"), None),
                ],
                None => vec![candidate(body.to_owned(), Some("解析失败".to_owned()))],
            });
        } else if let Some(values) = self.symbols.get(input) {
            values.clone()
        } else if let Some(body) = input.strip_prefix("uU") {
            return Some(self.radical_candidates(body));
        } else {
            return None;
        };
        Some(
            texts
                .into_iter()
                .map(|text| candidate(text, None))
                .collect(),
        )
    }

    fn radical_candidates(&self, code: &str) -> Vec<Candidate> {
        if code.is_empty() {
            return Vec::new();
        }
        let begin = self.radicals.partition_point(|(k, _, _)| k.as_str() < code);
        let mut hits: Vec<_> = self.radicals[begin..]
            .iter()
            .take_while(|(k, _, _)| k.starts_with(code))
            .collect();
        hits.sort_by(|a, b| {
            (b.0 == code)
                .cmp(&(a.0 == code))
                .then_with(|| b.2.cmp(&a.2))
                .then_with(|| a.1.cmp(&b.1))
        });
        let mut seen = std::collections::HashSet::new();
        hits.into_iter()
            .filter(|(_, word, _)| seen.insert(word.as_str()))
            .take(500)
            .map(|(_, word, _)| candidate(word.clone(), self.readings.get(word).cloned()))
            .collect()
    }
}
