use qingjian_core::{Language, Translation, Translator};

use crate::{Glossary, PersonalGlossary};

/// 随包释义表 + 个人释义表叠成一个译者：个人表优先（用户手改过的、云端兜底写的），查不到再查随包表；
/// 释义兜底学到的释义写进个人表，flush 时落盘。
pub struct LayeredTranslator {
    /// 个人释义表。
    personal: PersonalGlossary,

    /// 随包释义表。
    bundled: Glossary,
}

impl LayeredTranslator {
    pub fn new(bundled: Glossary, personal: PersonalGlossary) -> Self {
        Self { personal, bundled }
    }

    /// 随包释义表的条数。
    pub fn len(&self) -> usize {
        self.bundled.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bundled.is_empty()
    }

    /// 个人释义表的条数。
    pub fn personal_len(&self) -> usize {
        self.personal.len()
    }
}

impl Translator for LayeredTranslator {
    fn language(&self) -> Language {
        Translator::language(&self.bundled)
    }

    fn translate(&self, text: &str) -> Option<Translation> {
        self.personal
            .translate(text)
            .or_else(|| self.bundled.translate(text))
    }

    fn learn(&mut self, word: &str, translation: Translation) {
        self.personal.insert(word, translation);
    }

    fn flush(&mut self) {
        if let Err(error) = self.personal.save() {
            tracing::warn!(%error, "个人释义表保存失败");
        }
    }
}
