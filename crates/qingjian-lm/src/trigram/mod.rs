//! 词级三元模型（知意输入法的 `zh.ngram`，ARPA 回退，量化后整文件映射）：给「接在上文后面的一段文字」打 ln P，
//! 实现 Core 的 [`PhraseModel`](qingjian_core::sentence::PhraseModel)（选词重排里比整段候选）与 `LanguageModel`（整句词图，配二元模型插值）。
//!
//! 文件格式（小端，每个数组按 8 字节对齐）：
//!
//! ```text
//! char[8] "ZYNGRAM1"; u32 n1, n2, n3, 0
//! u32 word_off[n1 + 1]; u8 words[]                      词的 UTF-8，按字节序排好（编号即名次）
//! i16 p1[n1]; i16 b1[n1]                                 一元 log10 P 与回退权重，× 1/2048
//! u32 bi_start[n1 + 1]; u32 bi_w[n2]; i16 bi_p[n2]; i16 bi_b[n2]     二元，按 (前词, 后词) 分段
//! u32 tri_start[n2 + 1]; u32 tri_w[n3]; i16 tri_p[n3]                三元，按 (二元, 后词) 分段
//! ```
//!
//! 没有句首标记：一段文字从「没有前文」开始。模型的许可与来源见 `docs/notes/crate-notes.md`。

mod language_model;
mod model;
mod phrase;
mod search;
mod state;

pub use model::TrigramModel;

#[cfg(test)]
mod tests;
