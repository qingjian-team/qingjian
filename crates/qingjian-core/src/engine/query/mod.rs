//! 候选生成：按输入模式分派查询。各模式的实现在兄弟文件里，共用的候选构造留在这里。

use super::Engine;
use super::is_raw;
use crate::candidate::Candidate;
use crate::candidate::CandidateKind;
use crate::parser::ParseError;
use crate::ranking::Scored;
use std::time::Instant;

mod code;
mod converting;
mod english_tail;
mod generating;
mod lookup;
mod modes;
mod phonetic;
mod result;
mod snapshot;

pub(crate) use english_tail::EnglishTail;
pub use result::Query;
pub(super) use result::join_marked;
pub(super) use snapshot::QuerySnapshot;

impl Engine {
    /// 解析当前缓冲区并生成排好序的候选。**不带译文**，译文由 [`Self::annotate`] 补。
    ///
    /// 光标停在拼音中间时只按光标前的那段算候选（`ni|hao` 出 你），光标后的拼音留着，
    /// 上屏之后接着组句；见 [`crate::Composition::scope`]。
    pub fn query(&self) -> Result<Query, ParseError> {
        if let Some(query) = self.rime_query() {
            return Ok(query);
        }
        self.last_rescored.set(false);
        let mut query = match self.query_inner() {
            Ok(query) => query,
            Err(error) => {
                if !self
                    .custom_phrases
                    .iter()
                    .any(|p| p.enabled && p.code == self.composition.scope())
                {
                    return Err(error);
                }
                Query::custom_only(
                    &self.composition.typed_text(),
                    self.composition.cursor(),
                    self.shuangpin.is_some() || self.zhuyin,
                    self.shuangpin.is_some() && self.shuangpin_raw_preedit,
                    self.composition.scope(),
                    self.marked_rest(self.composition.rest()),
                )
            }
        };
        query.aux = self.aux_segment();
        // 辅码态只出命中码的词：自定义短语没有码，不出
        if self.aux_filter().is_none() {
            self.insert_custom_phrases(&mut query.candidates.items);
        }
        // 给输入日志留个摘要：上屏时才知道选了什么，这里才知道看到了什么
        let pinyin = match &query.correction {
            Some(correction) => correction.segmentation.joined("'"),
            None => join_marked(&query.segmentations, &query.tail),
        };
        *self.last_query.borrow_mut() = Some(QuerySnapshot {
            scope: self.composition.scope().to_owned(),
            pinyin,
            corrected: query.correction.is_some(),
            candidates: query
                .candidates
                .items
                .iter()
                .take(QuerySnapshot::MAX_CANDIDATES)
                .map(|c| c.text.clone())
                .collect(),
            rescored: self.last_rescored.get(),
        });

        if self.traditional
            && let Some(opencc) = &self.opencc
        {
            for candidate in &mut query.candidates.items {
                if matches!(
                    candidate.kind,
                    CandidateKind::Chinese | CandidateKind::Sentence | CandidateKind::Cloud
                ) {
                    let traditional_text = opencc.convert(&candidate.text);
                    self.traditional_map
                        .borrow_mut()
                        .insert(traditional_text.clone(), candidate.text.clone());
                    candidate.text = traditional_text;
                }
            }
        }

        Ok(query)
    }

    pub(super) fn query_inner(&self) -> Result<Query, ParseError> {
        let start = Instant::now();
        let keys = self.composition.scope();
        let rest = self.marked_rest(self.composition.rest());
        if self.english_mode {
            return Ok(self.query_english(keys, rest, start));
        }
        if self.modes().is_expression(keys, self.zhuyin) {
            return Ok(self.query_expression(keys, rest, start));
        }
        if self.modes().is_question(keys, self.zhuyin) {
            return Ok(self.query_question(keys, rest, start));
        }
        if is_raw(keys, self.modes(), self.shuangpin, self.zhuyin) {
            return Ok(self.query_raw(keys, rest, start));
        }
        // 形码与拼音是两条平行的管线，在进切分之前分岔。放在这里是为了让 `?` 问字与
        // `-` 直输段仍然先分派出去：形码下 `v` / `u` / `i` 是字根键，模式键已由 `modes()` 让位。
        match self.code.is_some() {
            // 只用形码：拼音侧整个不走（`[general] scheme = "none"`）
            true if !self.phonetic => Ok(self.query_code(keys, rest, start)),
            // 混输：两边都出候选
            true => self.query_mixed(keys, rest, start),
            false => self.query_phonetic(keys, rest, start),
        }
    }
}

/// 词库命中的中文候选。`aux_code` 是给壳显示的辅码：筛码时是命中当前码段的那条，没在筛码
/// （纯拼音态、辅码态空码段）时是词的首条码；没装码表或这个词没有码时是 `None`。
pub(super) fn chinese_candidate(item: &Scored<'_>, aux_code: Option<&str>) -> Candidate {
    Candidate {
        rime: None,
        text: item.hit.text.to_owned(),
        kind: CandidateKind::Chinese,
        syllables: item.hit.syllables().map(str::to_owned).collect(),
        reading: None,
        translation: None,
        aux_code: aux_code.map(str::to_owned),
    }
}

/// 排在开头的英文候选有几条（整段是英文词、不像拼音带出的英文补全）：整句插在它们后面。
pub(super) fn leading_english(items: &[Candidate]) -> usize {
    items
        .iter()
        .take_while(|c| c.kind == CandidateKind::English)
        .count()
}
