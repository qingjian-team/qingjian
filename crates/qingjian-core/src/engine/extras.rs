//! 附加候选：日期时间等快捷项、中英混输的英文词与补全、emoji 与颜文字、符号。

use super::{
    EMOJI_PER_WORD, EMOJI_SCAN, ENGLISH_COMPLETIONS, Engine, KAOMOJI_PER_WORD, KEYWORD_SCAN,
    Learner, MIN_COMPLETION_LETTERS,
};
use crate::candidate::{Candidate, CandidateKind};
use crate::emoji::EmojiTable;
use crate::shortcut;
use qingjian_dictionary::WordList;

impl Engine {
    /// 精确匹配自定义输入码时，数字键应选择候选。
    pub(super) fn has_custom_phrase(&self) -> bool {
        !self.english_mode
            && self
                .custom_phrases
                .iter()
                .any(|p| p.enabled && p.code == self.composition.scope())
    }

    /// 快捷短语模式：`i` 开头，匹配自定义短语表。
    #[allow(dead_code)]
    pub(super) fn has_shortcut_phrase(&self) -> bool {
        false
    }

    /// 快捷短语模式下获取匹配的短语候选。
    #[allow(dead_code)]
    pub(super) fn shortcut_phrase_candidates(&self) -> Vec<Candidate> {
        Vec::new()
    }

    /// 所有普通候选完成排序后，这个输入码的短语按权重从高到低占最前面几位（权重相同的按列表顺序），
    /// 后面跟短语同字的候选去掉，免得同一个词出两遍。
    pub(super) fn insert_custom_phrases(&self, items: &mut Vec<Candidate>) {
        if self.英语模式 {
            return;
        }
        let mut phrases: Vec<_> = self
            .custom_phrases
            .iter()
            .filter(|p| p.enabled && p.code == self.composition.scope())
            .collect();
        phrases.sort_by_key(|p| std::cmp::Reverse(p.weight));
        items.retain(|c| !phrases.iter().any(|p| p.text == c.text));
        items.splice(
            0..0,
            phrases
                .into_iter()
                .enumerate()
                .map(|(i, phrase)| Candidate {
                    text: phrase.text.clone(),
                    kind: CandidateKind::Custom(i + 1),
                    syllables: Vec::new(),
                    reading: None,
                    translation: None,
                    aux_code: None,
                }),
        );
    }