use qingjian_dictionary::{Dictionary, WordList};

use super::{Forgotten, Learner};
use crate::candidate::Candidate;
use crate::sentence::{Context, UserNgram};

/// 套在壳装配的学习器外面的一层：私密输入（密码框、浏览器无痕窗口）期间**写**全部吞掉、**读**照常转发，
/// 排序仍按已有的个人数据算，只是这一段什么都不记（见 [`crate::Engine::set_private`]）。
/// 撤销类写操作也一起吞：私密期间没记的，之后也不该被撤。
pub(in crate::engine) struct MutedLearner {
    inner: Box<dyn Learner>,

    /// 私密中：写操作不落到 `inner`。
    muted: bool,
}

impl MutedLearner {
    pub(in crate::engine) fn new(inner: Box<dyn Learner>) -> Self {
        Self {
            inner,
            muted: false,
        }
    }

    pub(in crate::engine) fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }

    pub(in crate::engine) fn replace(&mut self, inner: Box<dyn Learner>) {
        self.inner = inner;
    }

    /// 里面那个学习器（壳的落盘 / 导入 / 删词等管理操作直接对它）。
    pub(in crate::engine) fn inner(&self) -> &dyn Learner {
        self.inner.as_ref()
    }

    pub(in crate::engine) fn inner_mut(&mut self) -> &mut dyn Learner {
        self.inner.as_mut()
    }
}

impl Learner for MutedLearner {
    fn record(&mut self, candidate: &Candidate) {
        if !self.muted {
            self.inner.record(candidate);
        }
    }

    fn weight(&self, text: &str) -> u32 {
        self.inner.weight(text)
    }

    fn record_choice(&mut self, input: &str, text: &str) {
        if !self.muted {
            self.inner.record_choice(input, text);
        }
    }

    fn choice_weight(&self, input: &str, text: &str) -> u32 {
        self.inner.choice_weight(input, text)
    }

    fn record_raw(&mut self, input: &str) {
        if !self.muted {
            self.inner.record_raw(input);
        }
    }

    fn raw_count(&self, input: &str) -> u32 {
        self.inner.raw_count(input)
    }

    fn unrecord(&mut self, text: &str) {
        if !self.muted {
            self.inner.unrecord(text);
        }
    }

    fn unrecord_choice(&mut self, input: &str, text: &str) {
        if !self.muted {
            self.inner.unrecord_choice(input, text);
        }
    }

    fn unrecord_transition(&mut self, context: Context<'_>, word: &str, times: u32) {
        if !self.muted {
            self.inner.unrecord_transition(context, word, times);
        }
    }

    fn learn_word(&mut self, text: &str, syllables: &[String]) {
        if !self.muted {
            self.inner.learn_word(text, syllables);
        }
    }

    fn user_words(&self) -> Option<&Dictionary> {
        self.inner.user_words()
    }

    fn learn_english(&mut self, word: &str) {
        if !self.muted {
            self.inner.learn_english(word);
        }
    }

    fn user_english(&self) -> Option<&WordList> {
        self.inner.user_english()
    }

    fn record_transition(&mut self, context: Context<'_>, word: &str, times: u32) {
        if !self.muted {
            self.inner.record_transition(context, word, times);
        }
    }

    fn user_ngram(&self) -> Option<&UserNgram> {
        self.inner.user_ngram()
    }

    fn record_typo(&mut self, typed: &str, intended: &str) {
        if !self.muted {
            self.inner.record_typo(typed, intended);
        }
    }

    fn unrecord_typo(&mut self, typed: &str, intended: &str) {
        if !self.muted {
            self.inner.unrecord_typo(typed, intended);
        }
    }

    fn typo_count(&self, typed: &str, intended: &str) -> u32 {
        self.inner.typo_count(typed, intended)
    }

    // 删词是用户在候选窗口里明确要求的管理操作，不算「这段输入的学习」，私密中照做
    fn forget(&mut self, text: &str) -> Forgotten {
        self.inner.forget(text)
    }

    fn forget_english(&mut self, word: &str) -> bool {
        self.inner.forget_english(word)
    }

    fn flush(&mut self) {
        self.inner.flush();
    }
}
