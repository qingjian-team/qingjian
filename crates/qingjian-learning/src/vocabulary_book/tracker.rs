//! 输入法那一侧：Core 经 [`VocabularyTracker`] 记看到、上屏与例句，按今天记。

use jiff::civil::Date;
use qingjian_core::{Encounter, Language, VocabularySummary, VocabularyTracker};

use super::VocabularyBook;

fn today() -> Date {
    jiff::Zoned::now().date()
}

impl VocabularyTracker for VocabularyBook {
    /// 标了认识的不算生词。
    fn exposures(&self, language: Language, word: &str) -> u32 {
        self.get(language, word)
            .map_or(0, |entry| if entry.known { u32::MAX } else { entry.seen })
    }

    fn record_exposure(&mut self, language: Language, word: &str) {
        self.record_exposure_on(language, word, today());
    }

    fn record_commit(&mut self, language: Language, word: &str, used: bool) {
        self.record_commit_on(language, word, used, today());
    }

    fn record_encounter(&mut self, encounter: &Encounter) {
        let now = jiff::Zoned::now().datetime();
        let now = now.with().subsec_nanosecond(0).build().unwrap_or(now);
        self.record_encounter_at(encounter, now);
        // 当场追加：设置程序开着时间线时不用等下一次落盘
        if let Err(error) = self.append_encounters() {
            tracing::warn!(%error, "生词本时间线追加失败，落盘时再试");
        }
    }

    fn record_example(&mut self, language: Language, word: &str, source: &str, example: &str) {
        self.record_example_on(language, word, source, example, today());
    }

    /// 定时与停用时调：先合进设置程序那边的改动（标认识要尽快生效，所以没有新记录也看一眼），再写回。
    fn flush(&mut self) {
        self.refresh();
        if let Err(error) = self.save() {
            tracing::warn!(%error, "词汇记录保存失败");
        }
    }

    fn summary(&self, language: Language) -> VocabularySummary {
        self.summary_on(language, today())
    }
}
