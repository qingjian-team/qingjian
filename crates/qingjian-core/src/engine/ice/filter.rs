//! 候选过滤沿用青简的数据模型，保留雾凇的固定短语、置顶、长词与 Emoji 规则。

use super::tools::candidate;
use crate::candidate::{Candidate, CandidateKind};
use crate::engine::Engine;

impl Engine {
    pub(in crate::engine) fn filter_ice(&self, items: &mut Vec<Candidate>) {
        if !self.rime_ice_active() {
            return;
        }
        let Some(profile) = &self.ice else {
            return;
        };
        let typed = self.composition.typed_scope();
        if typed.contains('`')
            || items
                .first()
                .is_some_and(|c| c.kind == CandidateKind::Shortcut)
        {
            return;
        }
        let code = typed.to_ascii_lowercase();
        // 输入自带大写形式用于英文显示；候选匹配仍按小写查询。
        let english = profile.english.get(&code);
        if let Some(word) = english {
            let text = if typed.chars().take(2).all(|c| c.is_ascii_uppercase()) && typed.len() > 1 {
                word.to_ascii_uppercase()
            } else if typed.starts_with(|c: char| c.is_ascii_uppercase()) {
                let mut chars = word.chars();
                chars
                    .next()
                    .map(|c| format!("{}{}", c.to_ascii_uppercase(), chars.as_str()))
                    .unwrap_or_default()
            } else {
                word.to_owned()
            };
            items.retain(|c| {
                !(c.kind == CandidateKind::English && c.text.eq_ignore_ascii_case(&text))
            });
            let index = if profile.reduced_english.contains(&code) {
                profile.reduced_index.min(items.len())
            } else {
                usize::from(!items.is_empty())
            };
            items.insert(
                index,
                Candidate {
                    kind: CandidateKind::English,
                    ..candidate(text, None)
                },
            );
        }
        if let Some(words) = profile.mixed.get(&code) {
            for word in words.iter().rev() {
                items.insert(0, candidate(word.clone(), None));
            }
        }
        // 长词移到固定位置，不影响其余候选之间的顺序。
        let mut promoted = Vec::new();
        let first_length = items.first().map_or(0, |c| c.text.chars().count());
        let mut i = profile.long_index.min(items.len());
        while i < items.len() && promoted.len() < profile.long_count {
            if items[i].kind == CandidateKind::Chinese
                && items[i].text.chars().count() > first_length
                && i < profile.long_index + 51
            {
                promoted.push(items.remove(i));
            } else {
                i += 1;
            }
        }
        let index = profile.long_index.min(items.len());
        items.splice(index..index, promoted);
        if let Some(words) = profile.pins.get(&code) {
            for word in words.iter().rev() {
                let item = if let Some(i) = items.iter().position(|c| c.text == *word) {
                    items.remove(i)
                } else {
                    candidate(word.clone(), None)
                };
                items.insert(0, item);
            }
        }
        if let Some(words) = profile.phrases.get(&code) {
            for word in words.iter().rev() {
                items.insert(
                    0,
                    Candidate {
                        kind: CandidateKind::Custom(1),
                        ..candidate(word.clone(), None)
                    },
                );
            }
        }
        for item in items.iter_mut() {
            if let Some((text, comment)) = profile.corrections.get(&item.syllables.join(" "))
                && item.text == *text
            {
                item.reading = Some(comment.clone());
            }
        }
        let mut additions = Vec::new();
        for (index, item) in items.iter().take(5).enumerate() {
            if matches!(item.kind, CandidateKind::Chinese | CandidateKind::Sentence) {
                for emoji in profile.emoji.lookup(&item.text).iter().take(2) {
                    if !items.iter().any(|c| c.text == *emoji) {
                        additions.push((
                            index + 1,
                            Candidate {
                                text: emoji.clone(),
                                kind: CandidateKind::Emoji,
                                syllables: item.syllables.clone(),
                                reading: None,
                                translation: None,
                                aux_code: None,
                            },
                        ));
                    }
                }
            }
        }
        for (index, item) in additions.into_iter().rev() {
            items.insert(index, item);
        }
        let mut seen = std::collections::HashSet::new();
        items.retain(|c| seen.insert(c.text.clone()));
    }
}
