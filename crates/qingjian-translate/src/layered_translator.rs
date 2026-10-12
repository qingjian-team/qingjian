use qingjian_core::{Language, Translation, TranslationDifficulty, Translator};

use crate::{Glossary, LevelTable, PersonalGlossary};

/// 随包释义表 + 个人释义表叠成一个译者：个人表优先（用户手改过的、云端兜底写的），查不到再查随包表；
/// 释义兜底学到的释义写进个人表，flush 时落盘。
pub struct LayeredTranslator {
    /// 个人释义表。
    personal: PersonalGlossary,

    /// 随包释义表。
    bundled: Glossary,

    /// 可选的词汇等级；没有等级数据时保留原释义池。
    levels: LevelTable,
}

impl LayeredTranslator {
    pub fn new(bundled: Glossary, personal: PersonalGlossary) -> Self {
        Self {
            personal,
            bundled,
            levels: LevelTable::default(),
        }
    }

    pub fn with_levels(mut self, levels: LevelTable) -> Self {
        self.levels = levels;
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
        self.personal
            .translate(text)
            .or_else(|| self.bundled.translate(text))
    }

    fn translate_for_learning(
        &self,
        text: &str,
        difficulty: TranslationDifficulty,
        seed: Option<u64>,
    ) -> Option<Translation> {
        if difficulty == TranslationDifficulty::All && seed.is_none() {
            return self.translate(text);
        }
        let mut senses = self
            .personal
            .senses(text)
            .or_else(|| self.bundled.senses(text))?;
        if self.language() == Language::English && difficulty != TranslationDifficulty::All {
            let matching: Vec<_> = senses
                .iter()
                .filter(|sense| {
                    self.levels
                        .level(&sense.text)
                        .is_some_and(|level| difficulty.matches(level))
                })
                .cloned()
                .collect();
            // 未分级不等于高级；没有目标等级时保留可用译词，避免候选旁突然空白。
            if !matching.is_empty() {
                senses = matching;
            }
        }
        let mut unique = Vec::with_capacity(senses.len());
        for sense in senses {
            if !unique
                .iter()
                .any(|other: &qingjian_core::Sense| other.text == sense.text)
            {
                unique.push(sense);
            }
        }
        if let Some(mut state) = seed {
            // 局部 Fisher–Yates：仅抽候选窗需要的两条，且不改变原释义表。
            for i in 0..unique.len().min(Translation::MAX_SENSES) {
                state = state.wrapping_add(0x9e3779b97f4a7c15);
                let mut random = state;
                random = (random ^ (random >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
                random = (random ^ (random >> 27)).wrapping_mul(0x94d049bb133111eb);
                random ^= random >> 31;
                let j = i + random as usize % (unique.len() - i);
                unique.swap(i, j);
            }
        }
        Some(Translation::new(self.language(), unique))
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

    fn translator() -> LayeredTranslator {
        LayeredTranslator::new(
            Glossary::parse(
                Language::English,
                "高兴\tadj. happy\tadj. delighted\tadj. elated\tadj. unknown\n",
            )
            .unwrap(),
            PersonalGlossary::in_memory(Language::English),
        )
        .with_levels(
            LevelTable::parse(
                "# levels\tA1\tA2\tB1\tB2\tC1\tC2\nhappy\tA1\ndelighted\tB1\nelated\tC2\n",
            )
            .unwrap(),
        )
    }

    #[test]
    fn difficulty_can_reach_the_third_sense_and_keeps_unknown_words_ungraded() {
        let t = translator();
        for (difficulty, expected) in [
            (TranslationDifficulty::Beginner, "happy"),
            (TranslationDifficulty::Intermediate, "delighted"),
            (TranslationDifficulty::Advanced, "elated"),
        ] {
            let result = t
                .translate_for_learning("高兴", difficulty, Some(0))
                .unwrap();
            assert_eq!(result.senses().len(), 1);
            assert_eq!(result.senses()[0].text, expected);
        }
        assert_eq!(
            t.translate_for_learning("没有", TranslationDifficulty::Advanced, Some(0)),
            None
        );
    }

    #[test]
    fn random_selection_is_repeatable_per_seed_and_reaches_all_senses() {
        let t = translator();
        let mut seen = std::collections::HashSet::new();
        for seed in 0..128 {
            let result = t
                .translate_for_learning("高兴", TranslationDifficulty::All, Some(seed))
                .unwrap();
            assert_eq!(
                result,
                t.translate_for_learning("高兴", TranslationDifficulty::All, Some(seed))
                    .unwrap()
            );
            assert_eq!(result.senses().len(), Translation::MAX_SENSES);
            assert_ne!(result.senses()[0].text, result.senses()[1].text);
            seen.extend(result.senses().iter().map(|s| s.text.clone()));
        }
        assert_eq!(seen.len(), 4);
        assert_eq!(
            t.translate_for_learning("高兴", TranslationDifficulty::All, None),
            t.translate("高兴")
        );
    }

    #[test]
    fn no_matching_level_falls_back_and_personal_glossary_stays_authoritative() {
        let mut t = translator();
        let personal = Translation::new(
            Language::English,
            t.bundled.senses("高兴").unwrap()[..1].to_vec(),
        );
        t.learn("高兴", personal.clone());
        assert_eq!(
            t.translate_for_learning("高兴", TranslationDifficulty::Advanced, None),
            Some(personal)
        );
        let t = LayeredTranslator::new(
            Glossary::parse(
                Language::Japanese,
                "高兴\tadj. 嬉しい|うれしい\tadj. 楽しい|たのしい\n",
            )
            .unwrap(),
            PersonalGlossary::in_memory(Language::Japanese),
        );
        let result = t
            .translate_for_learning("高兴", TranslationDifficulty::Advanced, Some(4))
            .unwrap();
        assert_eq!(result.language, Language::Japanese);
        assert!(result.senses().iter().all(|s| s.reading.is_some()));
    }

    #[test]
    fn mapped_glossaries_keep_the_full_learning_pool() {
        let t = translator();
        let path =
            std::env::temp_dir().join(format!("qingjian-learning-pool-{}.qj", std::process::id()));
        t.bundled
            .write_qj(&path, &qingjian_format::Metadata::default())
            .unwrap();
        let mapped = LayeredTranslator::new(
            Glossary::from_path(Language::English, &path).unwrap(),
            PersonalGlossary::in_memory(Language::English),
        )
        .with_levels(t.levels.clone());
        assert_eq!(mapped.bundled.senses("高兴").unwrap().len(), 4);
        for seed in 0..16 {
            assert_eq!(
                mapped.translate_for_learning("高兴", TranslationDifficulty::All, Some(seed)),
                t.translate_for_learning("高兴", TranslationDifficulty::All, Some(seed))
            );
        }
        assert_eq!(
            mapped
                .translate_for_learning("高兴", TranslationDifficulty::Advanced, None)
                .unwrap()
                .senses()[0]
                .text,
            "elated"
        );
        std::fs::remove_file(path).unwrap();
    }
}
