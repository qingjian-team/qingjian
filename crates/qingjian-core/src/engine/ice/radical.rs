//! 全拼后的部件辅码筛选，结果仍通过青简候选提交和学习。

use crate::candidate::{CandidateKind, CandidateList};
use crate::engine::{Engine, Query};
use std::collections::HashSet;
use std::time::Instant;

impl Engine {
    pub(super) fn query_ice_radical(&self, start: Instant) -> Option<Query> {
        if !self.rime_ice_active() {
            return None;
        }
        let typed = self.composition.typed_scope();
        let (pinyin, radical) = typed.split_once('`')?;
        let profile = self.ice.as_ref()?;
        let begin = profile
            .radicals
            .partition_point(|(code, _, _)| code.as_str() < radical);
        let allowed: HashSet<&str> = profile.radicals[begin..]
            .iter()
            .take_while(|(code, _, _)| code.starts_with(radical))
            .map(|(_, word, _)| word.as_str())
            .collect();
        let lower = pinyin.to_ascii_lowercase();
        let (mut items, timings) = self
            .query_phonetic(&lower, String::new(), start)
            .ok()
            .map(|q| (q.candidates.items, q.timings))
            .unwrap_or_default();
        let filter_start = Instant::now();
        items.retain(|item| {
            radical.is_empty()
                || (item.kind == CandidateKind::Chinese
                    && item
                        .text
                        .chars()
                        .next()
                        .is_some_and(|c| allowed.contains(&item.text[..c.len_utf8()])))
        });
        let mut query = Query::custom_only(
            &self.composition.typed_text(),
            self.composition.cursor(),
            false,
            false,
            &typed,
            self.marked_rest(self.composition.rest()),
        );
        query.candidates = CandidateList { items };
        query.timings = timings;
        query.timings.rank += filter_start.elapsed();
        Some(query)
    }
}
