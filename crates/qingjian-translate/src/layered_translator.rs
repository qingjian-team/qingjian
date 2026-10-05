use qingjian_core::{Language, Translation, Translator};

use crate::{Glossary, LevelGate, PersonalGlossary};

/// 随包释义表 + 个人释义表叠成一个译者：个人表优先（用户手改过的、云端兜底写的），查不到再查随包表；
/// 释义兜底学到的释义写进个人表，flush 时落盘。随包表可带等级门槛（[`Self::with_level_gate`]），
/// 个人表不受门槛影响。
pub struct LayeredTranslator {
    /// 个人释义表。
    personal: PersonalGlossary,

    /// 随包释义表。
    bundled: Glossary,

    /// 随包表的等级门槛；没配就是不过滤。
    level_gate: Option<LevelGate>,
}

impl LayeredTranslator {
    pub fn new(bundled: Glossary, personal: PersonalGlossary) -> Self {
        Self {
            personal,
            bundled,
            level_gate: None,
        }
    }

    /// 给随包释义表上等级门槛：等级低于阈值的译词不显示，个人释义表不受影响。
    pub fn with_level_gate(mut self, gate: LevelGate) -> Self {
        self.level_gate = Some(gate);
        self
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
        match self.personal.translate(text) {
            Some(translation) => Some(translation),
            None => {
                let mut translation = self.bundled.translate(text)?;
                if let Some(gate) = self.level_gate.as_ref()
                    && !gate.passes(&mut translation)
                {
                    return None;
                }
                Some(translation)
            }
        }
    }

    /// 译文里最难的等级；没带门槛（或没有等级表）时没有等级概念。
    fn sense_level(&self, translation: &Translation) -> Option<usize> {
        self.level_gate
            .as_ref()
            .map(|gate| gate.difficulty(translation))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LevelTable;
    use qingjian_core::Sense;

    const LEVELS: &str = "# levels\tA1\tA2\tB1\nthe\tA1\nis\tA1\nimportant\tA2\nsignificant\tB1\n";
    const BUNDLED: &str = "# 词\t译文\n是\tv. be\n重要\tadj. important\n";

    fn sense(text: &str) -> Translation {
        Translation::new(
            Language::English,
            vec![Sense {
                part_of_speech: None,
                text: text.to_owned(),
                reading: None,
                fresh: false,
            }],
        )
    }

    #[test]
    fn gate_filters_bundled_but_not_personal() {
        let bundled = Glossary::parse(Language::English, BUNDLED).unwrap();
        let mut personal = PersonalGlossary::in_memory(Language::English);
        personal.insert("是", sense("be"));
        let gate = LevelGate::new(LevelTable::parse(LEVELS).unwrap(), "b1").unwrap();
        let translator = LayeredTranslator::new(bundled, personal).with_level_gate(gate);
        // 个人释义表直通：译词再简单也显示
        assert!(translator.translate("是").is_some());
        // 随包表的简单译词被门槛滤掉
        assert!(translator.translate("重要").is_none());
    }
}
