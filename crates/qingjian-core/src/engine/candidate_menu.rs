//! 候选右键菜单背后的操作：置顶要的输入码与词、降权与取消降权，以及查询时按降权往后挪。

use super::{Engine, Learner};
use crate::candidate::{Candidate, CandidateKind};
use crate::{Language, Sense, Translation};

impl Engine {
    /// 给不走中文 Composition 的候选排序（如越南语）：选择权重优先，再应用该输入串下的降权。
    /// 调用方应给 `input` 加模式前缀，避免与拼音学习记录混用。
    pub fn rank_external_candidates(&self, input: &str, candidates: &mut [Candidate]) {
        candidates.sort_by_key(|candidate| {
            std::cmp::Reverse(self.learner.choice_weight(input, &candidate.text))
        });
        candidates.sort_by_key(|candidate| self.learner.demotion(input, &candidate.text));
    }

    /// 提高外部候选在指定输入串下的权重。
    pub fn promote_external_candidate(&mut self, input: &str, text: &str) {
        self.learner.record_choice(input, "", text);
        self.learner.flush();
    }

    pub fn demote_external_candidate(&mut self, input: &str, text: &str) {
        self.learner.demote(input, text);
        self.learner.flush();
    }

    pub fn undemote_external_candidate(&mut self, input: &str, text: &str) -> bool {
        let changed = self.learner.undemote(input, text);
        self.learner.flush();
        changed
    }

    pub fn external_candidate_demotion(&self, input: &str, text: &str) -> u32 {
        self.learner.demotion(input, text)
    }

    /// 这个候选存进学习数据与自定义短语时用的写法：繁体输出开着时换回简体。
    pub(super) fn stored_text(&self, candidate: &Candidate) -> String {
        // 短语候选显示的是换过时间变量的文字，存的是带 `{year}` 这类变量的原文：按位置找回原文，取消置顶才认得
        if let CandidateKind::Custom(position) = candidate.kind
            && let Some(phrase) = self.scope_phrases().get(position.wrapping_sub(1))
        {
            return phrase.text.clone();
        }
        self.traditional
            .restore(&candidate.text)
            .unwrap_or_else(|| candidate.text.clone())
    }

    /// 把这个候选置顶要建的自定义短语：(输入码, 词)。输入码就是当前作用域的字母；
    /// 英文模式、作用域不是合法输入码（带数字、分隔符或太长）时给 `None`，壳不出「置顶」。
    pub fn pin_target(&self, candidate: &Candidate) -> Option<(String, String)> {
        let code = self.composition.scope();
        (!self.english_mode && crate::custom_phrase::valid_code(code))
            .then(|| (code.to_owned(), self.stored_text(candidate)))
    }

    /// 候选译文可编辑时所用的词与学习语言。没有学习语言时返回 `None`。
    pub fn translation_target(&self, candidate: &Candidate) -> Option<(String, Language)> {
        let language = self.translator.language();
        (language != Language::Chinese && candidate.kind != CandidateKind::English)
            .then(|| (self.stored_text(candidate), language))
    }

    /// 覆盖候选在当前学习语言下的个人译文，并立即落盘。
    pub fn set_candidate_translation(&mut self, candidate: &Candidate, text: &str) -> bool {
        let Some((word, _)) = self.translation_target(candidate) else {
            return false;
        };
        self.set_translation(&word, text)
    }

    /// 保存模型生成的候选译文；模型给了可识别词性就一并保存，没给时仍保留译词。
    pub fn set_generated_candidate_translation(
        &mut self,
        candidate: &Candidate,
        generated: &str,
    ) -> bool {
        let Some((word, language)) = self.translation_target(candidate) else {
            return false;
        };
        let generated = generated.trim();
        let (part_of_speech, text) = match generated.split_once('\t') {
            Some((part_of_speech, text)) => match part_of_speech.trim().parse() {
                Ok(part_of_speech) => (Some(part_of_speech), text.trim()),
                Err(_) => (None, text.trim()),
            },
            None => (None, generated),
        };
        if text.is_empty() {
            return false;
        }
        let mut senses = self
            .translator
            .translate(&word)
            .map(|translation| translation.senses().to_vec())
            .unwrap_or_default();
        match senses.first_mut() {
            Some(sense) => {
                sense.text = text.to_owned();
                if part_of_speech.is_some() {
                    sense.part_of_speech = part_of_speech;
                }
                sense.fresh = false;
            }
            None => senses.push(Sense {
                part_of_speech,
                text: text.to_owned(),
                reading: None,
                fresh: false,
            }),
        }
        self.translator
            .learn(&word, Translation::new(language, senses));
        self.translator.flush();
        true
    }

    /// 按已经还原成存储字形的词覆盖个人译文。弹出编辑框前先保存 `word`，即使 IMK 在弹窗期间重建会话也能写入。
    pub fn set_translation(&mut self, word: &str, text: &str) -> bool {
        let text = text.trim();
        let language = self.translator.language();
        if language == Language::Chinese || word.is_empty() {
            return false;
        }
        if text.is_empty() {
            return false;
        }
        let mut senses = self
            .translator
            .translate(word)
            .map(|translation| translation.senses().to_vec())
            .unwrap_or_default();
        match senses.first_mut() {
            Some(sense) => {
                sense.text = text.to_owned();
                sense.fresh = false;
            }
            None => senses.push(Sense {
                part_of_speech: None,
                text: text.to_owned(),
                reading: None,
                fresh: false,
            }),
        }
        self.translator
            .learn(word, Translation::new(language, senses));
        self.translator.flush();
        true
    }

    /// 查询当前学习语言下最终生效的译文（个人释义优先）。
    pub fn candidate_translation(&self, text: &str) -> Option<Translation> {
        let restored = self.traditional.restore(text);
        self.translator
            .translate(restored.as_deref().unwrap_or(text))
    }

    /// 降得了权的：自定义短语按权重排，不走降权。
    pub fn demotable(&self, candidate: &Candidate) -> bool {
        !matches!(candidate.kind, CandidateKind::Custom(_)) && !self.composition.scope().is_empty()
    }

    /// 在当前输入串下把这个候选降权一次（往后挪一格）。私密输入、关了学习时照做。
    pub fn demote(&mut self, candidate: &Candidate) {
        let text = self.stored_text(candidate);
        let input = self.composition.scope().to_owned();
        self.learner.demote(&input, &text);
    }

    /// 取消当前输入串下这个候选的降权。返回原来有没有降过。
    pub fn undemote(&mut self, candidate: &Candidate) -> bool {
        let text = self.stored_text(candidate);
        let input = self.composition.scope().to_owned();
        self.learner.undemote(&input, &text)
    }

    /// 当前输入串下这个候选降了几次。
    pub fn demotion(&self, candidate: &Candidate) -> u32 {
        self.learner
            .demotion(self.composition.scope(), &self.stored_text(candidate))
    }

    /// 排好的候选里降过权的往后挪：降了 `n` 次就排到原来在它后面第 `n` 个候选之后。
    pub(super) fn apply_demotions(&self, items: &mut Vec<Candidate>) {
        let input = self.composition.scope();
        let mut keyed: Vec<_> = items
            .drain(..)
            .enumerate()
            .map(|(index, candidate)| {
                let demotion = self.learner.demotion(input, &candidate.text) as usize;
                ((index + demotion, demotion > 0), candidate)
            })
            .collect();
        keyed.sort_by_key(|(key, _)| *key);
        items.extend(keyed.into_iter().map(|(_, candidate)| candidate));
    }
}
