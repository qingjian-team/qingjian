//! 九宫格查询：数字串切成若干种形状，每个位置带同码的全部写法，词级与整句都走多写法查询。
//! 设计见 `docs/design/nine-key.md`。

mod commit;
mod conversion;
mod display;
mod interjection;
mod pin;

use std::time::Instant;

use qingjian_dictionary::Match;

use super::chinese_candidate;
use crate::candidate::{Candidate, CandidateList};
use crate::engine::{Engine, Learner, MAX_CANDIDATES, Query, Timings};
use crate::nine_key::{self, Shape};
use crate::parser::umlaut_spelling;
use crate::ranking::{self, Scored};

/// 词级查询看前几种形状。
const WORD_SHAPES: usize = 3;

impl Engine {
    /// 九宫格方案的候选。与形码一样是平行管线：拼写纠错、模糊音、英文尾段、表达式 / 问字都不走。
    pub(super) fn query_nine_key(&self, keys: &str, rest: String, start: Instant) -> Query {
        let (shapes, tail) = nine_key::segment(keys);
        let parse = start.elapsed();

        let start = Instant::now();
        let mut scored = Vec::new();
        for shape in shapes.iter().take(WORD_SHAPES) {
            self.score_shape(shape, &mut scored);
        }
        let lookup = start.elapsed();

        let start = Instant::now();
        let log_total = (self.total_frequency() as f64).max(1.0).ln();
        ranking::rank(&mut scored, MAX_CANDIDATES, |item| {
            let hit = &item.hit;
            // 按输入串记的选择用候选自己的拼音当键，与全拼共用一份学习数据
            let input: String = hit.syllables().collect();
            let choice = self.learner.choice_weight(&input, hit.text);
            (choice, self.word_log_prob(hit, log_total))
        });
        let mut items: Vec<Candidate> = Vec::with_capacity(scored.len());
        for item in &scored {
            // 不同形状会命中同一个词
            if !items.iter().any(|c: &Candidate| {
                c.text == item.hit.text
                    && c.syllables
                        .iter()
                        .map(String::as_str)
                        .eq(item.hit.syllables())
            }) {
                items.push(chinese_candidate(item, None));
            }
        }
        if let Some(sentence) = self.nine_key_sentence(&shapes, &items) {
            items.insert(0, sentence);
        }
        if let Some((at, candidate)) = self.nine_key_interjection(keys, shapes.first()) {
            items.retain(|c| c.text != candidate.text);
            items.insert(at.min(items.len()), candidate);
        }
        // 叹词插好再排：用户选过、看过的读法（`6` 偏向 n）压过叹词的位置（`6` 的 哦）
        self.prefer_unpinned(&mut items, shapes.first());
        // 数字串不认 `rq` 这些（`77` 多半是别的词）；拼音列或长按把字母都定下来了才算
        let letters: String = keys.chars().filter(|&c| c != '\'').collect();
        if letters.bytes().all(|b| b.is_ascii_lowercase()) {
            self.insert_shortcuts(&mut items, &letters);
        }
        self.insert_emoji(&mut items);
        self.insert_keyword_shortcuts(&mut items);
        let rank = start.elapsed();

        let pinyin_choices = display::pinyin_choices(keys, shapes.first(), &items);
        let typed_display = shapes
            .first()
            .map(|shape| display::marked(shape, items.first(), tail));
        Query {
            segmentations: Vec::new(),
            spelled: 0,
            candidates: CandidateList { items },
            tail: tail.to_owned(),
            text: self.composition.text().to_owned(),
            cursor: self.composition.cursor(),
            rest,
            decoded_keys: true,
            shuangpin_raw_preedit: false,
            shuangpin_raw_window: false,
            typed_display,
            typed_case: None,
            pinyin_choices,
            correction: None,
            held: String::new(),
            aux: None,
            timings: Timings {
                parse,
                lookup,
                rank,
            },
        }
    }

    /// 一种形状下的词级命中：整段，以及各前缀（逐词上屏用，只收音节数正好的词）。
    fn score_shape<'a>(&'a self, shape: &Shape, scored: &mut Vec<Scored<'a>>) {
        let count = shape.positions.len();
        let covered = |positions: usize| -> usize {
            shape.positions[..positions].iter().map(|p| p.len).sum()
        };
        let last = &shape.positions[count - 1];
        for hit in self.lookup_all(&shape.patterns(count)) {
            let full_last = last.complete
                && hit
                    .syllables()
                    .nth(count - 1)
                    .is_some_and(|syllable| last.alternatives.iter().any(|a| a == syllable));
            scored.push(Scored {
                hit,
                full_last,
                coverage: covered(count),
                abbreviated: shape.abbreviated(count),
                weight: self.learner.weight(hit.text),
                penalty: 0.0,
                tone_cost: 0,
                split: false,
            });
        }
        for prefix_len in (1..count).rev() {
            let mut patterns = shape.patterns(prefix_len);
            // 前缀里的位置都不是输入末尾：完整音节按整音节查
            for (position, alternatives) in shape.positions.iter().zip(&mut patterns) {
                for pattern in alternatives {
                    pattern.complete = position.complete;
                }
            }
            for hit in self.lookup_exact_all(&patterns) {
                scored.push(Scored {
                    hit: Match {
                        exact: false,
                        ..hit
                    },
                    full_last: true,
                    coverage: covered(prefix_len),
                    abbreviated: shape.abbreviated(prefix_len),
                    weight: self.learner.weight(hit.text),
                    penalty: 0.0,
                    tone_cost: 0,
                    split: false,
                });
            }
        }
    }
}

impl Engine {
    /// 光标后剩下的键（[`Engine::move_cursor_to_marked`] 挪进中间后）读出来，与拼音行一个键一个字母：
    /// 挪之前这里读成什么还照那样读（[`Self::keep_nine_key_reading`]），没有的取第一种写法。
    /// 这段不查词，读法只是给人看位置。
    pub(in crate::engine) fn nine_key_rest(&self, rest: &str) -> String {
        // 拼音行在这段前面自己接 `'`；选定的音节后面自动接的那个不再算一次
        let keys = rest.trim_start_matches('\'');
        let (shapes, tail) = nine_key::segment(keys);
        let Some(shape) = shapes.first() else {
            return rest.to_owned();
        };
        let offset = self.composition.scope().len() + (rest.len() - keys.len());
        let kept = self
            .nine_key_unpinned
            .as_ref()
            .filter(|(buffer, _)| buffer == self.composition.text())
            .map(|(_, syllables)| syllables.as_slice())
            .unwrap_or_default();
        let mut parts: Vec<&str> = shape
            .positions
            .iter()
            .map(|position| {
                pin::kept_at(kept, offset + position.start)
                    .and_then(|spelling| {
                        position
                            .alternatives
                            .iter()
                            .find(|a| spelling.starts_with(a.as_str()))
                    })
                    .unwrap_or(&position.alternatives[0])
                    .as_str()
            })
            .collect();
        if !tail.is_empty() {
            parts.push(tail);
        }
        parts
            .into_iter()
            .map(umlaut_spelling)
            .collect::<Vec<_>>()
            .join("'")
    }
}
