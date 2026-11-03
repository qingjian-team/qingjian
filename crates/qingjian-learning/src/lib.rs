//! 用户词频学习与输入日志的落盘。
//!
//! [`FrequencyLearner`] 存聚合数据（选择次数、输入串选择、用户词、个人英文词、个人 n-gram，都是 TSV）；
//! [`InputLog`] 逐条记上屏（jsonl），给离线回归评测与个人模型用；[`UsageStats`] 按小时数打了多少字、打得多快（`usage-hourly.tsv`，每次上屏当场追加），
//! [`VocabularyBook`] 记学习语言的译词看过 / 上屏过几次（`user-vocab.tsv`，候选里标生词的依据），
//! 连同生词本的时间线（`vocab-log.tsv`，每次上屏有学习语言说法的中文一行）；[`PanelUsage`] 记面板里的符号、表情各用了几次（`panel-usage.tsv`），排「常用」；
//! [`ClipboardHistory`] 存剪贴板历史（`clipboard.jsonl`，图片另存在 `clipboard-images/`），[`PhraseBook`] 存常用语（`phrases.json`，带输入码的换成自定义短语）。

#![forbid(unsafe_code)]

mod clipboard;
mod encounter_log;
mod error;
mod frequency_learner;
mod input_log;
mod panel_usage;
mod phrase_book;
mod usage_stats;
mod vocabulary_book;

pub use clipboard::{
    CLIPBOARD_FILE, CLIPBOARD_IMAGE_DIR, ClipboardEntry, ClipboardHistory, DEFAULT_CLIPBOARD_LIMIT,
};
pub use encounter_log::{ENCOUNTER_LOG_FILE, LoggedEncounter};
pub use error::LearningError;
pub use frequency_learner::FrequencyLearner;
pub use input_log::InputLog;
pub use panel_usage::{PANEL_USAGE_FILE, PanelUsage};
pub use phrase_book::{DEFAULT_GROUP, MAX_CODE_CHARS, PHRASE_BOOK_FILE, Phrase, PhraseBook};
pub use usage_stats::{USAGE_FILE, UsageStats, figures};
pub use vocabulary_book::{VocabularyBook, VocabularyEntry};
