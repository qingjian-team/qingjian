//! 用户词频学习与输入日志的落盘。
//!
//! [`FrequencyLearner`] 存聚合数据（选择次数、输入串选择、用户词、个人英文词、个人 n-gram，都是 TSV）；
//! [`InputLog`] 逐条记上屏（jsonl），给离线回归评测与个人模型用；[`UsageStats`] 按小时数打了多少字、打得多快（`usage-hourly.tsv`，每次上屏当场追加），
//! [`VocabularyBook`] 记学习语言的译词看过 / 上屏过几次（`user-vocab.tsv`，候选里标生词的依据）。

mod error;
mod frequency_learner;
mod input_log;
mod usage_stats;
mod vocabulary_book;

pub use error::LearningError;
pub use frequency_learner::FrequencyLearner;
pub use input_log::InputLog;
pub use usage_stats::{USAGE_FILE, UsageStats};
pub use vocabulary_book::VocabularyBook;
