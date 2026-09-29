use qingjian_dictionary::{Dictionary, WordList};

use super::{Forgotten, Learner};
use crate::candidate::Candidate;
use crate::engine::mnemonic;
use crate::sentence::{Context, UserNgram};

/// 套在壳装配的学习器外面的一层：私密输入（密码框、浏览器无痕窗口）期间或用户关掉学习后，**写**全部吞掉、
/// **读**照常转发——排序仍按已有的个人数据算，只是不再记新的（见 [`crate::Engine::set_private`] 与
/// [`crate::Engine::set_learning`]）。撤销类写操作也一起吞：没记的，之后也不该被撤。
pub(in crate::engine) struct MutedLearner {
    inner: Box<dyn Learner>,

    /// 私密输入中。
    private: bool,

    /// 用户关掉了学习（`[general] learning = false`）。
    disabled: bool,
}

impl MutedLearner {
    pub(in crate::engine) fn new(inner: Box<dyn Learner>) -> Self {
        Self {
            inner,
            private: false,
            disabled: false,
        }
    }

    pub(in crate::engine) fn set_private(&mut self, private: bool) {
        self.private = private;
    }

    pub(in crate::engine) fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
    }

    /// 两个开关任一生效就不写。
    fn muted(&self) -> bool {
        self.private || self.disabled
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
        if !self.muted() {
            self.inner.record(candidate);
        }
    }

    fn weight(&self, text: &str) -> u32 {
        self.inner.weight(text)
    }

    fn record_choice(&mut self, input: &str, text: &str) {
        if !self.muted() {
            self.inner.record_choice(input, text);
        }
    }

    fn choice_weight(&self, input: &str, text: &str) -> u32 {
        self.inner.choice_weight(input, text)
    }

    fn record_raw(&mut self, input: &str) {
        if !self.muted() {
            self.inner.record_raw(input);
        }
    }

    fn raw_count(&self, input: &str) -> u32 {
        self.inner.raw_count(input)
    }

    fn unrecord(&mut self, text: &str) {
        if !self.muted() {
            self.inner.unrecord(text);
        }
    }

    fn unrecord_choice(&mut self, input: &str, text: &str) {
        if !self.muted() {
            self.inner.unrecord_choice(input, text);
        }
    }

    fn unrecord_transition(&mut self, context: Context<'_>, word: &str, times: u32) {
        if !self.muted() {
            self.inner.unrecord_transition(context, word, times);
        }
    }

    fn learn_word(&mut self, text: &str, syllables: &[String]) {
        if !self.muted() {
            self.inner.learn_word(text, syllables);
        }
    }

    fn user_words(&self) -> Option<&Dictionary> {
        self.inner.user_words()
    }

    fn learn_english(&mut self, word: &str) {
        // BIP-39 助记词表里的词不落盘：钱包助记词全由这 2048 个常见英文词组成，不这么挡的话，
        // 手打一遍助记词就会把每个词都当成「新学到的英文词」明文记进 user-english.tsv，
        // 之后还会作为候选重新冒出来。见 `crate::engine::mnemonic` 顶部的说明。
        if !self.muted() && !mnemonic::is_bip39_word(word) {
            self.inner.learn_english(word);
        }
    }

    fn user_english(&self) -> Option<&WordList> {
        self.inner.user_english()
    }

    fn record_transition(&mut self, context: Context<'_>, word: &str, times: u32) {
        if !self.muted() {
            self.inner.record_transition(context, word, times);
        }
    }

    fn user_ngram(&self) -> Option<&UserNgram> {
        self.inner.user_ngram()
    }

    fn record_typo(&mut self, typed: &str, intended: &str) {
        if !self.muted() {
            self.inner.record_typo(typed, intended);
        }
    }

    fn unrecord_typo(&mut self, typed: &str, intended: &str) {
        if !self.muted() {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 只数 `record_raw` 被调了几次的桩。
    struct Counting(u32);

    impl Learner for Counting {
        fn record(&mut self, _candidate: &Candidate) {}

        fn weight(&self, _text: &str) -> u32 {
            0
        }

        fn record_raw(&mut self, _input: &str) {
            self.0 += 1;
        }

        fn raw_count(&self, _input: &str) -> u32 {
            self.0
        }
    }

    #[test]
    fn private_and_disabled_each_block_writes_but_not_reads() {
        let mut learner = MutedLearner::new(Box::new(Counting(0)));
        learner.record_raw("a");
        assert_eq!(learner.raw_count("a"), 1);

        learner.set_disabled(true);
        learner.record_raw("a");
        assert_eq!(learner.raw_count("a"), 1, "关掉学习后不再写，读照常");

        learner.set_disabled(false);
        learner.set_private(true);
        learner.record_raw("a");
        assert_eq!(learner.raw_count("a"), 1, "私密中同样不写");

        learner.set_private(false);
        learner.record_raw("a");
        assert_eq!(learner.raw_count("a"), 2, "两个开关都关了才恢复");
    }
}
