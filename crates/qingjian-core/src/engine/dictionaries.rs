//! 词库的挂接：主词库、附加词库、三元模型补出的词，基础词库只出单字的开关，屏蔽词，以及查词时把它们拢在一起。

use super::{Engine, Learner};
use qingjian_dictionary::Dictionary;

impl Engine {
    pub fn dictionary(&self) -> &Dictionary {
        &self.dictionary
    }

    /// 设置屏蔽词：候选文本里含有其中任何一个就整条丢掉（搜狗 也挡掉 搜狗拼音）。
    ///
    /// 自定义短语不受影响——那是用户自己写下的原文，屏蔽词管的是词库带进来的东西。
    pub fn set_blocked_words(&mut self, words: Vec<String>) {
        self.blocked = words.into_iter().filter(|w| !w.is_empty()).collect();
    }

    /// 换掉全部附加词库（导入、移除、开关之后）。格子缓存随之作废。
    pub fn set_extra_dictionaries(&mut self, dictionaries: Vec<Dictionary>) {
        self.extra_dictionaries = dictionaries;
        self.forget_span_cache();
    }

    pub fn extra_dictionaries(&self) -> &[Dictionary] {
        &self.extra_dictionaries
    }

    /// 挂上从三元模型词表补出的词（`qingjian_dictionary::supplement`），`None` 摘掉。格子缓存随之作废。
    /// 基础词库关着（[`Self::set_base_words`]）时它也只出单字。
    pub fn set_model_words(&mut self, mut words: Option<Dictionary>) {
        if let Some(words) = &mut words {
            words.set_chars_only(self.dictionary.is_chars_only());
        }
        self.model_words = words;
        self.forget_span_cache();
    }

    /// 开关基础词库里的词：关掉时主词库与三元模型补出的词只出单字，词与整句里的词全靠附加词库（用户导入、下载的）。
    /// 单字照常出，关掉也打得出字。格子缓存随之作废。
    pub fn set_base_words(&mut self, on: bool) {
        if self.dictionary.is_chars_only() != on {
            return;
        }
        self.dictionary.set_chars_only(!on);
        if let Some(words) = &mut self.model_words {
            words.set_chars_only(!on);
        }
        self.forget_span_cache();
    }

    pub fn model_words(&self) -> Option<&Dictionary> {
        self.model_words.as_ref()
    }

    /// 查词用的全部词库：主词库、附加词库、内置的符号与 emoji 名字、通讯录、用户词。
    pub(super) fn all_dictionaries(&self) -> Vec<&Dictionary> {
        let mut all = Vec::with_capacity(self.extra_dictionaries.len() + 2);
        all.push(&self.dictionary);
        all.extend(self.extra_dictionaries.iter());
        all.extend(self.model_words.as_ref());
        all.extend(self.companion_words);
        all.extend(self.contact_book().map(crate::ContactBook::dictionary));
        if let Some(user) = self.learner.user_words() {
            all.push(user);
        }
        all
    }

    /// 全部词库的词频之和，词频归一化成概率时用。补充词库不算：它的词频是按主词库换算过来的，
    /// 算进去会把所有兜底打分一起压低。
    pub(super) fn total_frequency(&self) -> u64 {
        self.all_dictionaries()
            .iter()
            .filter(|d| !d.is_secondary())
            .map(|d| d.total_frequency())
            .sum()
    }
}
