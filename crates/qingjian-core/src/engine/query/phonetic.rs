//! 拼音侧的候选生成：切分、拼写纠错、词级查找与排序，再补上整句、英文、快捷与 emoji 候选。

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use super::chinese_candidate;
use super::{Query, join_marked_typed};
use crate::candidate::{Candidate, CandidateList};
use crate::engine::Timings;
use crate::engine::decoded::EngineDecoded;
use crate::engine::{
    Engine, Learner, MAX_CANDIDATES, abbreviated_count, choice_key, pattern_key,
    segment_longest_prefix,
};
use crate::parser::{ParseError, Segmentation};
use crate::ranking::Scored;
use crate::{correction, parser, ranking, sentence};
use qingjian_dictionary::Match;

impl Engine {
    /// 拼音侧（全拼 / 双拼 / 注音）的候选生成：整段作用域是一串读音。
    pub(super) fn query_phonetic(
        &self,
        keys: &str,
        rest: String,
        start: Instant,
    ) -> Result<Query, ParseError> {
        if self.shuangpin_full_pinyin && self.shuangpin.is_some() && !self.zhuyin {
            let variants = self.mixed_decodings(keys);
            let mut timings = Timings {
                parse: start.elapsed(),
                ..Timings::default()
            };
            let mut primary = None;
            let mut items = Vec::new();
            for decoded in variants {
                if let Ok(mut query) = self.query_phonetic_decoded(
                    keys,
                    rest.clone(),
                    Instant::now(),
                    Some(EngineDecoded::Shuangpin(decoded.clone())),
                ) {
                    timings.parse += query.timings.parse;
                    timings.lookup += query.timings.lookup;
                    timings.rank += query.timings.rank;
                    for (rank, candidate) in std::mem::take(&mut query.candidates.items)
                        .into_iter()
                        .enumerate()
                    {
                        let coverage = if candidate.syllables.is_empty() {
                            keys.len()
                        } else {
                            self.mixed_match(&decoded, &candidate).map_or(0, |m| m.0)
                        };
                        items.push((coverage, rank, candidate));
                    }
                    if primary.is_none() {
                        primary = Some(query);
                    }
                }
            }
            let mut query = primary.ok_or(ParseError::NoSegmentation)?;
            let merge = Instant::now();
            // 先按原始键的覆盖长度，再交错各读法的同名次候选；防止一种读法占满上限把全拼候选截掉。
            items.sort_by_key(|item| (std::cmp::Reverse(item.0), item.1));
            let mut seen = HashSet::new();
            query.candidates.items = items
                .into_iter()
                .map(|(_, _, c)| c)
                .filter(|c| seen.insert(c.text.clone()))
                .take(MAX_CANDIDATES)
                .collect();
            timings.rank += merge.elapsed();
            query.timings = timings;
            return Ok(query);
        }
        self.query_phonetic_decoded(keys, rest, start, self.decode(keys))
    }

    fn query_phonetic_decoded(
        &self,
        keys: &str,
        rest: String,
        start: Instant,
        decoded: Option<EngineDecoded>,
    ) -> Result<Query, ParseError> {
        // 双拼先解成全拼（音节间已用 `'` 连好，切分没有歧义），之后与全拼同路；解不动的键当尾巴
        let scope: &str = decoded.as_ref().map_or(keys, |d| d.pinyin());
        // 末尾是英文词（`woxiangxuehaorust`）：拼音候选与整句只按头段算，尾段整个跟在整句后面。
        // 整段也能读成拼音时（`database`、`…rust` 当简拼）两种读法比分，英文赢了才按头段算，
        // 输了整段按拼音读、英文读法排在拼音整句后面
        let english_tail = if decoded.is_none() {
            self.split_english_tail(keys)
        } else {
            None
        };
        let head_wins = english_tail
            .as_ref()
            .is_some_and(|t| !t.competes || self.mixed_beats_plain(keys, t));
        let parsed = match (&decoded, &english_tail) {
            (Some(d), _) => d
                .segmentation()
                .map(|s| (vec![s], d.tail()))
                .ok_or(ParseError::NoSegmentation),
            (None, Some(tail)) if head_wins => {
                parser::segment(&keys[..tail.head_len]).map(|s| (s, ""))
            }
            _ => segment_longest_prefix(keys),
        };
        // 连第一个字母都切不动（`impor`）：拼音这边没戏，但英文词 / 补全、快捷候选还可以有
        let (segmentations, tail) = match parsed {
            Ok(parsed) => parsed,
            Err(error) => {
                let mut items = Vec::new();
                self.insert_english(&mut items, true);
                self.insert_shortcuts(&mut items, keys);
                if items.is_empty() {
                    return Err(error);
                }
                return Ok(Query {
                    segmentations: Vec::new(),
                    candidates: CandidateList { items },
                    tail: keys.to_owned(),
                    text: self.composition.text().to_owned(),
                    cursor: self.composition.cursor(),
                    rest,
                    decoded_keys: self.shuangpin.is_some() || self.zhuyin,
                    shuangpin_raw_preedit: self.shuangpin.is_some() && self.shuangpin_raw_preedit,
                    typed_display: decoded.as_ref().map(|d| d.marked()),
                    correction: None,
                    aux: None,
                    timings: Timings {
                        parse: start.elapsed(),
                        lookup: Duration::ZERO,
                        rank: Duration::ZERO,
                    },
                });
            }
        };
        // 拼音「不像话」时试拼写纠错；纠正生效则按纠正后的切分查词，原串只用来记学习与显示
        let unlikely = correction::unlikely_pinyin(segmentations.first(), tail)
            || correction::trailing_single_letter(segmentations.first());
        let correction = if unlikely {
            self.active_correction(scope)
        } else {
            None
        };
        let (segmentations, tail): (Vec<Segmentation>, &str) = match &correction {
            Some(c) => (vec![c.segmentation.clone()], ""),
            None => (segmentations, tail),
        };
        let parse = start.elapsed();

        let start = Instant::now();
        let mut scored = Vec::new();
        // 不同切分共享很多前缀（`zh g d o…` 的各种切法前几段一样），同一次查询里同一个模式只查一遍
        let mut memo: HashMap<String, Vec<Match<'_>>> = HashMap::new();
        for segmentation in &segmentations {
            let mut patterns = segmentation.patterns();
            let count = patterns.len();
            let last = &segmentation.syllables[count - 1];
            // 最后一个音节即使打完了也可能还没打完（`xia` 可能是 `xiang` 的前缀），按前缀查；双拼两键就是定局
            if last.complete && decoded.is_none() && parser::is_syllable_prefix(&last.text) {
                patterns[count - 1].complete = false;
            }
            // 词级候选只按敲的原样与模糊音查，敲错变体只进整句词图（它的候选从那边插进来）：
            // 词级排序把音节数对得上的排最前，敲错命中的词（`kaif` → 咖啡）会把更长的原样词挤到后面
            let expanded = self.fuzzy.expand(&patterns);
            let positions = expanded.positions();
            let abbreviated = abbreviated_count(&patterns);
            // 没有替代写法时每条命中都是敲的原音节，`penalty` 直接给 0（单字母简拼能命中几万条）
            let hits = self.lookup_all(&positions);
            scored.reserve(hits.len());
            for hit in hits {
                let full_last = last.complete
                    && hit.syllables().nth(count - 1) == Some(patterns[count - 1].text);
                scored.push(Scored {
                    hit,
                    full_last,
                    coverage: segmentation.letters(),
                    abbreviated,
                    weight: self.learner.weight(hit.text),
                    penalty: expanded.penalty(hit.syllables()),
                });
            }
            // 输入的前缀也出候选（`kaifazhe` → 开发、开），否则长句没法逐词上屏。
            // 只收音节数正好等于前缀长度的词，更长的词会与输入后面的音节冲突。
            // 前缀不含最后一个位置，因此可复用上面的扩展结果。
            for prefix_len in (1..count).rev() {
                let prefix = &patterns[..prefix_len];
                let prefix_letters: usize = prefix.iter().map(|p| p.text.len()).sum();
                let hits = memo
                    .entry(pattern_key(prefix))
                    .or_insert_with(|| self.lookup_exact_all(&positions[..prefix_len]));
                let abbreviated = abbreviated_count(prefix);
                for hit in hits.iter().copied() {
                    scored.push(Scored {
                        // 对整个输入来说它不是精确命中，只是覆盖了前面一部分
                        hit: Match {
                            exact: false,
                            ..hit
                        },
                        full_last: true,
                        coverage: prefix_letters,
                        abbreviated,
                        weight: self.learner.weight(hit.text),
                        penalty: expanded.penalty(hit.syllables()),
                    });
                }
            }
        }
        let lookup = start.elapsed();

        let start = Instant::now();
        // 再往后翻也翻不到的候选不必再造：单字母简拼能命中两万个词，排完序只留前面这些。
        // 同输入串（候选覆盖的那段字母）下选过的优先；上下文是上一个上屏的词（句首为 None）：
        // `ba` 在「做了」后面出 吧、句首出 把
        let log_total = (self.total_frequency() as f64).max(1.0).ln();
        let letters = choice_key(scope, scope.len());
        ranking::rank(&mut scored, MAX_CANDIDATES, |item| {
            let hit = &item.hit;
            // 纠错生效时覆盖的是纠正后的字母，换算回原串再查「这个输入串下选过什么」
            let covered = correction
                .as_ref()
                .map_or(item.coverage, |c| c.edit.to_original(item.coverage));
            let choice = letters
                .get(..covered)
                .map_or(0, |input| self.learner.choice_weight(input, hit.text));
            let log_prob = sentence::transition_log_prob(
                &*self.language_model,
                self.personal(),
                self.chain.context(),
                hit.text,
                sentence::fallback_log_prob(hit.frequency, log_total),
            );
            (choice, log_prob)
        });
        // 辅码态：词库候选按码段**反向**过滤（逐个问「有没有以码段开头的码」），无码词直接隐藏；
        // 命中的按「完全匹配码 > 码长降序 > 原词频序」重排（stable sort 保住 rank 排好的原序）。
        // 码段为空（刚敲下触发键）时不过滤，候选与纯拼音态一模一样。
        let aux_code = self.aux_filter();
        let mut items: Vec<Candidate> = Vec::with_capacity(scored.len());
        match aux_code {
            // 没在筛码：首条码只在显示开关开着或已在辅码态时挂，否则不逐候选查码
            None => items.extend(scored.into_iter().map(|item| {
                let first = if self.aux_show || self.aux_code.is_some() {
                    self.matching_code(item.hit.text, "")
                } else {
                    None
                };
                chinese_candidate(&item, first)
            })),
            Some(code) => {
                let mut kept: Vec<(usize, &str)> = scored
                    .iter()
                    .enumerate()
                    .filter_map(|(index, item)| {
                        self.matching_code(item.hit.text, code)
                            .map(|hit| (index, hit))
                    })
                    .collect();
                kept.sort_by(|a, b| {
                    (b.1.len() == code.len())
                        .cmp(&(a.1.len() == code.len()))
                        .then_with(|| b.1.len().cmp(&a.1.len()))
                });
                items.extend(
                    kept.into_iter()
                        .map(|(index, hit)| chinese_candidate(&scored[index], Some(hit))),
                );
            }
        }
        // 附加候选（英文尾段与补全、快捷、整句、emoji）都没有码：辅码筛词时一律不出
        if aux_code.is_none() {
            // 中文优先：整句先进去占第一，英文词紧跟其后（第二）；关掉时英文词先进、整句排在开头的英文后面
            if self.chinese_first {
                self.insert_sentence(
                    &mut items,
                    &segmentations,
                    correction.is_none(),
                    english_tail.as_ref().filter(|_| correction.is_none()),
                    head_wins,
                );
                self.insert_english(&mut items, unlikely);
            } else {
                self.insert_english(&mut items, unlikely);
                self.insert_sentence(
                    &mut items,
                    &segmentations,
                    correction.is_none(),
                    english_tail.as_ref().filter(|_| correction.is_none()),
                    head_wins,
                );
            }
            // 快捷候选按敲的键认（`rq` 日期），双拼下也是
            self.insert_shortcuts(&mut items, keys);
            self.insert_emoji(&mut items);
        }
        let rank = start.elapsed();

        // 按头段算时英文尾段不参与拼音候选，显示上跟在切分后面：`wo'xiang'xue'hao'rust`
        let tail = english_tail
            .as_ref()
            .filter(|_| head_wins)
            .map_or(tail, |t| &keys[t.head_len..]);
        let typed_display = decoded.as_ref().map(|d| d.marked()).or_else(|| {
            // 中文模式下 Shift 敲的大写：匹配按小写算，拼音行仍按敲的样子显示（`Cpan`）
            (correction.is_none() && self.composition.has_shifted())
                .then(|| join_marked_typed(&self.composition.typed_scope(), &segmentations, tail))
        });
        Ok(Query {
            segmentations,
            candidates: CandidateList { items },
            tail: tail.to_owned(),
            text: self.composition.text().to_owned(),
            cursor: self.composition.cursor(),
            rest,
            decoded_keys: self.shuangpin.is_some() || self.zhuyin,
            shuangpin_raw_preedit: self.shuangpin.is_some() && self.shuangpin_raw_preedit,
            typed_display,
            correction,
            aux: self.aux_segment(),
            timings: Timings {
                parse,
                lookup,
                rank,
            },
        })
    }
}
