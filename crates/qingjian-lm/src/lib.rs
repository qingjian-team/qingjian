//! 整句转换用的词级语言模型：实现 Core 的 `LanguageModel` trait。
//! 从 TSV 加载一元 / 二元计数（`from_paths`），或直接映射 `.qj`（`from_path`，`dict-convert pack lm` 生成，启动近零耗时）。
//!
//! TSV 由 `tools/dict-convert bigram` 从语料统计得到：
//!
//! ```text
//! lm-unigram.tsv    词\t计数        （`<s>` 是句首标记，计数为句子数）
//! lm-bigram.tsv     前词\t后词\t计数
//! ```
//!
//! 概率是插值平滑：P(w|v) = λ·c(v,w)/c(v) + (1-λ)·c(w)/N。模型里没有的词交回 Core 用词库词频兜底。

mod bigram_model;
mod error;
mod successor;
mod word_entry;

pub use bigram_model::{BigramModel, SENTENCE_START};
pub use error::LmError;
