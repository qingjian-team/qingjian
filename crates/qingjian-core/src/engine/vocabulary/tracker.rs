use super::VocabularySummary;
use crate::candidate::Language;

/// 词汇记录的落盘方。实现放学习 crate，Core 只查次数、送事件。
pub trait VocabularyTracker: Send {
    /// 译词 `word`（学习语言 `language`）在候选窗口里被看到过几轮（一轮组句最多记一次）。
    fn exposures(&self, language: Language, word: &str) -> u32;

    /// 用户上屏时这条译词在屏幕上：记一次看到。
    fn record_exposure(&mut self, language: Language, word: &str);

    /// 用户上屏了带这条译词的候选：`used` 是直接把译词打出去了（修饰键 + 数字），否则只是上屏了中文、看着译词。
    fn record_commit(&mut self, language: Language, word: &str, used: bool);

    /// 落盘。壳在停用输入法时调用，激活期间也定时调；失败只记日志。
    fn flush(&mut self) {}

    /// 某种学习语言的汇总（偏好设置「统计」页）。
    fn summary(&self, _language: Language) -> VocabularySummary {
        VocabularySummary::default()
    }
}

/// 不记词汇：所有译词都算看熟了，候选里不标生词。
#[derive(Debug, Clone, Copy, Default)]
pub struct NoVocabularyTracker;

impl VocabularyTracker for NoVocabularyTracker {
    fn exposures(&self, _language: Language, _word: &str) -> u32 {
        u32::MAX
    }

    fn record_exposure(&mut self, _language: Language, _word: &str) {}

    fn record_commit(&mut self, _language: Language, _word: &str, _used: bool) {}
}
