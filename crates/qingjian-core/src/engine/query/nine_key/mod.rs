//! 九宫格查询：数字串构造成字节偏移音节 DAG，词级查词与整句转换都在同一张 DAG 上走多写法查询。
//! 设计见 `docs/design/nine-key.md`。

mod commit;
mod conversion;
mod display;
mod interjection;
mod pin;

use std::time::Instant;

use qingjian_dictionary::{Match, SyllablePattern};

use super::chinese_candidate;
use crate::candidate::{Candidate, CandidateList};
use crate::engine::{Engine, Learner, MAX_CANDIDATES, Query, Timings};
use crate::nine_key::{self, Lattice};
use crate::parser::umlaut_spelling;
use crate::ranking::{self, Scored};
use crate::sentence::MAX_WORD_SYLLABLES;

impl Engine {
    /// 九宫格方案的候选。与形码一样是平行管线：拼写纠错、模糊音、英文尾段、表达式 / 问字都不走。
    pub(super) fn query_nine_key(&self, keys: &str, rest: String, start: Instant) -> Query {
        let (lattice, tail) = nine_key::lattice(keys);
        let best_shape = lattice.best_shape();
        let parse = start.elapsed();

        let start = Instant::now();
        let mut scored = Vec::new();
        if !lattice.is_empty() {
            self.score_lattice(&lattice, &mut scored);
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
            // 不同路径可能命中同一个词
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
        if let Some(sentence) = self.nine_key_sentence(keys, &lattice, &items) {
            items.insert(0, sentence);
        }
        if let Some((at, candidate)) = self.nine_key_interjection(keys, best_shape.as_ref()) {
            items.retain(|c| c.text != candidate.text);
            items.insert(at.min(items.len()), candidate);
        }
        // 叹词插好再排：用户选过、看过的读法（`6` 偏向 n）压过叹词的位置（`6` 的 哦）
        self.prefer_unpinned(&mut items, best_shape.as_ref());
        // 数字串不认 `rq` 这些（`77` 多半是别的词）；拼音列或长按把字母都定下来了才算
        let letters: String = keys.chars().filter(|&c| c != '\'').collect();
        if letters.bytes().all(|b| b.is_ascii_lowercase()) {
            self.insert_shortcuts(&mut items, &letters);
        }
        self.insert_emoji(&mut items);
        self.insert_keyword_shortcuts(&mut items);
        let rank = start.elapsed();

        let pinyin_choices =
            display::pinyin_choices(keys, best_shape.as_ref(), &items, |spelling| {
                self.nine_key_syllable_weight(spelling)
            });
        let typed_display = display::marked(&lattice, best_shape.as_ref(), items.first(), tail);
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

    /// 某个音节在词库与用户习惯里的最高权重：拼音列里没进当前候选的音节按它排，避免退回字母序。
    fn nine_key_syllable_weight(&self, spelling: &str) -> (u32, u32) {
        let pattern = [SyllablePattern {
            text: spelling,
            complete: true,
        }];
        let mut best = (0u32, 0u32);
        for hit in self.dictionary.lookup_exact(&pattern) {
            let weight = self.learner.weight(hit.text);
            if (weight, hit.frequency) > best {
                best = (weight, hit.frequency);
            }
        }
        best
    }

    /// 沿 DAG 从起点出发做词级查词：整段命中的走全模式（含末尾补全），前缀命中的按精确音节数查（逐词上屏用）；
    /// 借助词库 `has_longer` 剪枝，覆盖全部合法切法且不重复查共享前缀。
    fn score_lattice<'a>(&'a self, lattice: &Lattice, scored: &mut Vec<Scored<'a>>) {
        let mut patterns: Vec<Vec<SyllablePattern<'_>>> = Vec::with_capacity(MAX_WORD_SYLLABLES);
        self.walk_word_lattice(lattice, 0, 0, 0, &mut patterns, scored);
    }

    fn walk_word_lattice<'a, 'l>(
        &'a self,
        lattice: &'l Lattice,
        vertex: usize,
        covered: usize,
        abbreviated: usize,
        patterns: &mut Vec<Vec<SyllablePattern<'l>>>,
        scored: &mut Vec<Scored<'a>>,
    ) {
        for edge in lattice.edges_at(vertex) {
            let next_covered = covered + edge.position.len;
            if edge.next == lattice.end() {
                patterns.push(edge.position.patterns());
                let count = patterns.len();
                for hit in self.lookup_all(patterns) {
                    let full_last = edge.position.complete
                        && hit.syllables().nth(count - 1).is_some_and(|syllable| {
                            edge.position.alternatives.iter().any(|a| a == syllable)
                        });
                    scored.push(Scored {
                        hit,
                        full_last,
                        coverage: next_covered,
                        abbreviated,
                        weight: self.learner.weight(hit.text),
                        penalty: 0.0,
                        tone_cost: 0,
                        split: false,
                    });
                }
                patterns.pop();
                continue;
            }
            // 前缀里的位置都不是输入末尾：完整音节按整音节查
            let mut step = edge.position.patterns();
            for pattern in &mut step {
                pattern.complete = edge.position.complete;
            }
            patterns.push(step);
            let (hits, has_longer) = self.lookup_exact_all_step(patterns);
            for hit in hits {
                scored.push(Scored {
                    hit: Match {
                        exact: false,
                        ..hit
                    },
                    full_last: true,
                    coverage: next_covered,
                    abbreviated,
                    weight: self.learner.weight(hit.text),
                    penalty: 0.0,
                    tone_cost: 0,
                    split: false,
                });
            }
            if has_longer && patterns.len() < MAX_WORD_SYLLABLES {
                let next_abbreviated = abbreviated + usize::from(!edge.position.complete);
                self.walk_word_lattice(
                    lattice,
                    edge.next,
                    next_covered,
                    next_abbreviated,
                    patterns,
                    scored,
                );
            }
            patterns.pop();
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
        let (lattice, tail) = nine_key::lattice(keys);
        let Some(shape) = lattice.best_shape() else {
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
