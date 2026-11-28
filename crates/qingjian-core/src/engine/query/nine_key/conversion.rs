//! 九宫格的整句：把全部合法边界放进同一张字节偏移音节 DAG，只做一次统一 Viterbi，
//! 不再按结构束宽截断形状或重复转 12 遍。

use qingjian_dictionary::SyllablePattern;

use crate::candidate::{Candidate, CandidateKind};
use crate::engine::{Engine, Learner};
use crate::nine_key::{self, Lattice};
use crate::sentence::{self, Conversion, LatticeEdge, SearchInput, SentenceWord};

/// 九宫格统一 DAG 的束宽：多读法与多边界汇在同一字节偏移上，略宽于全拼的 8 让竞争切法留得住。
const LATTICE_BEAM_WIDTH: usize = 16;

impl Engine {
    /// 整句：在字节偏移 DAG 上做一次统一 Viterbi；规则同全拼（至少两个音节、路径不止一个词、没有占位音节）。
    pub(super) fn nine_key_sentence(
        &self,
        keys: &str,
        lattice: &Lattice,
        items: &[Candidate],
    ) -> Option<Candidate> {
        let conversion = self.convert_nine_key(keys, lattice)?;
        if conversion.word_count() < 2 || conversion.has_placeholder() {
            return None;
        }
        if items
            .iter()
            .any(|c| c.text == conversion.text && c.syllables == conversion.syllables)
        {
            return None;
        }
        Some(Candidate {
            text: conversion.text,
            kind: CandidateKind::Sentence,
            syllables: conversion.syllables,
            reading: None,
            translation: None,
            aux_code: None,
        })
    }

    /// 在字节偏移 DAG 上做一次统一整句转换；上屏时重算词序列也走这里。
    ///
    /// 末尾位置若本身已是完整音节（如 `7426` → `piao / qian / shan`），整句按精确完整音节查，
    /// 不再借末尾前缀展开成更长音节（避免 `退票` → `腿上`、`收藏里` → `收藏夹`、`渠道` → `入党`）；
    /// 词级查词仍走末尾前缀，保证半截输入的补全候选照常出现。
    fn convert_nine_key(&self, keys: &str, lattice: &Lattice) -> Option<Conversion> {
        if !lattice.has_multi_syllable_path() {
            return None;
        }
        let end = lattice.end();
        let trailing_exact: Vec<Vec<Option<Vec<String>>>> = lattice
            .edges()
            .iter()
            .map(|outgoing| {
                outgoing
                    .iter()
                    .map(|edge| {
                        (edge.next == end && edge.position.complete && edge.position.prefix)
                            .then(|| {
                                let span = &keys
                                    [edge.position.start..edge.position.start + edge.position.len];
                                nine_key::complete_syllables(span)
                            })
                            .filter(|list| !list.is_empty())
                    })
                    .collect()
            })
            .collect();
        let dictionaries = self.all_dictionaries();
        let graph: Vec<Vec<LatticeEdge<'_>>> = lattice
            .edges()
            .iter()
            .zip(&trailing_exact)
            .map(|(outgoing, overrides)| {
                outgoing
                    .iter()
                    .zip(overrides)
                    .map(|(edge, exact)| {
                        let patterns = match exact {
                            Some(syllables) => syllables
                                .iter()
                                .map(|s| SyllablePattern::complete(s))
                                .collect(),
                            None => edge.position.patterns(),
                        };
                        LatticeEdge {
                            to: edge.next,
                            patterns,
                            penalty: 0.0,
                        }
                    })
                    .collect()
            })
            .collect();
        let beam = self.search_limits.beam_width.max(LATTICE_BEAM_WIDTH);
        let input = SearchInput::new(
            &dictionaries,
            &*self.language_model,
            self.personal(),
            |text| self.learner.weight(text),
            |_, _| 0.0,
        )
        .with_limits(self.search_limits);
        sentence::convert_lattice(&input, &graph, beam, &mut self.span_cache.borrow_mut())
    }

    /// 九宫格缓冲区的整句词序列（文本对得上才算）。
    pub(in crate::engine) fn nine_key_sentence_words(
        &self,
        candidate: &Candidate,
    ) -> Option<Vec<SentenceWord>> {
        let keys = self.composition.scope();
        let (lattice, _) = nine_key::lattice(keys);
        let conversion = self.convert_nine_key(keys, &lattice)?;
        (conversion.text == candidate.text).then_some(conversion.words)
    }
}
