//! 九宫格的整句：边界歧义只有语言模型分得清（`tian qi hen` 与 `tian shi en`），所以多种形状各转一次取最高分。

use crate::candidate::{Candidate, CandidateKind};
use crate::engine::{Engine, Learner};
use crate::nine_key::{self, Shape};
use crate::sentence::{self, Conversion, SearchInput, SentenceWord};

/// 参与整句转换的形状数。
const SENTENCE_SHAPES: usize = 12;

impl Engine {
    /// 整句：各形状各转一次取分最高的；规则同全拼（至少两个音节、路径不止一个词、没有占位音节）。
    pub(super) fn nine_key_sentence(
        &self,
        shapes: &[Shape],
        items: &[Candidate],
    ) -> Option<Candidate> {
        let conversion = self.convert_nine_key(shapes)?;
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

    /// 各形状里得分最高的整句转换；上屏时重算词序列也走这里。
    fn convert_nine_key(&self, shapes: &[Shape]) -> Option<Conversion> {
        let dictionaries = self.all_dictionaries();
        shapes
            .iter()
            .take(SENTENCE_SHAPES)
            .filter(|shape| shape.positions.len() >= 2)
            .filter_map(|shape| {
                let input = SearchInput::new(
                    &dictionaries,
                    &*self.language_model,
                    self.personal(),
                    |text| self.learner.weight(text),
                    |_, _| 0.0,
                )
                .with_limits(self.search_limits);
                sentence::convert(
                    &input,
                    &shape.patterns(shape.positions.len()),
                    &mut self.span_cache.borrow_mut(),
                )
            })
            .max_by(|a, b| a.score.total_cmp(&b.score))
    }

    /// 九宫格缓冲区的整句词序列（文本对得上才算）。
    pub(in crate::engine) fn nine_key_sentence_words(
        &self,
        candidate: &Candidate,
    ) -> Option<Vec<SentenceWord>> {
        let (shapes, _) = nine_key::segment(self.composition.scope());
        let conversion = self.convert_nine_key(&shapes)?;
        (conversion.text == candidate.text).then_some(conversion.words)
    }
}
