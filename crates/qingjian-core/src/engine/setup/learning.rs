//! 学习与五笔组词的注入、开关和只读访问。

use crate::engine::{Engine, Learner};

impl Engine {
    /// 学习开关（`[general] learning`）：关掉后不再记词频、用户词、个人 n-gram 与敲错表，已学的照常参与排序；
    /// 私密输入是另一个独立的开关（[`Self::set_private`]）。
    pub fn set_learning(&mut self, enabled: bool) {
        if self.learner.set_disabled(!enabled) {
            self.chain.clear_code_words();
        }
    }

    /// 五笔连续上屏组词；不改变拼音学习规则、已有词与词频学习。
    pub fn set_auto_phrase(&mut self, enabled: bool, max_chars: usize) {
        let max_chars = max_chars.clamp(2, 10);
        if self.auto_phrase != enabled || self.auto_phrase_max_chars != max_chars {
            self.chain.clear_code_words();
        }
        self.auto_phrase = enabled;
        self.auto_phrase_max_chars = max_chars;
    }

    pub fn with_learner(mut self, learner: Box<dyn Learner>) -> Self {
        self.learner.replace(learner);
        self.forget_span_cache();
        self
    }

    pub fn learner(&self) -> &dyn Learner {
        self.learner.inner()
    }

    /// 拿到可变的 Learner 就当它要改：格子缓存一起作废。
    pub fn learner_mut(&mut self) -> &mut dyn Learner {
        self.forget_span_cache();
        self.learner.inner_mut()
    }
}
