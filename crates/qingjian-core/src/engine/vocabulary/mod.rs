//! 学习语言的词汇记录：用户在候选窗口里见过哪些译词、上屏过哪些、用修饰键 + 数字直接打出过哪些。
//!
//! 「生词」就从这里判断：一条译词被看到的轮次还不到 [`FRESH_UNTIL`]，候选里就标出来（[`crate::candidate::Sense::fresh`]），
//! 看熟了自然不标。「看到」按上屏那一刻屏幕上的那一页算（`Engine::note_displayed` 记下当前页，上屏时才记进词汇表），
//! 逐键刷新时一闪而过的候选不算：用户真正读候选是在要选的时候。
//! Core 只判断与记录，按词落盘与汇总由实现做（`qingjian-learning` 的 `VocabularyBook`）；缺省 [`NoVocabularyTracker`] 不记、也不标生词。

mod level_count;
mod summary;
mod tracker;

pub use level_count::LevelCount;
pub use summary::VocabularySummary;
pub use tracker::{NoVocabularyTracker, VocabularyTracker};

/// 见过这么多轮之前算生词。
pub const FRESH_UNTIL: u32 = 3;
