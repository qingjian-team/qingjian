//! 连续形码上屏组词，与拼音学习索引隔离。

use crate::{Engine, Learner};

impl Engine {
    pub(super) fn learn_code_phrase(&mut self, text: &str) {
        if !self.auto_phrase
            || self.is_contact(text)
            || !self.code.as_ref().is_some_and(|scheme| scheme.wubi_phrases)
        {
            self.chain.clear_code_words();
            return;
        }
        if !text.chars().all(|character| {
            matches!(character as u32,
            0x3400..=0x4dbf | 0x4e00..=0x9fff | 0xf900..=0xfaff | 0x20000..=0x323af)
        }) {
            self.chain.clear_code_words();
            return;
        }
        let words = self.chain.push_code_word(text, self.auto_phrase_max_chars);
        let scheme = self.code.as_ref().expect("已检查形码方案");
        for word in words {
            let Some(code) = scheme.phrase_code(&word) else {
                continue;
            };
            if scheme
                .table
                .lookup(&code, usize::MAX)
                .iter()
                .any(|hit| hit.exact && hit.text == word)
            {
                continue;
            }
            self.learner.learn_code_word(&scheme.key, &word, &code);
        }
    }
}
