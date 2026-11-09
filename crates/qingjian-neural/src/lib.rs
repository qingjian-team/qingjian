//! 本地神经模型的推理（candle，知意的 ONNX 版走 ONNX Runtime），两套模型各一个目录：
//!
//! - `hanzhang`：含章，自训的字级 Transformer。给「光标前文 + 候选」打分（[`CharScorer`]），或按拼音逐字生成整句（[`P2c`]）。
//! - `zhiyi`：知意的选词模型（feature `zhiyi`）。给候选重排，并按音节出字。
//!
//! 缺省 CPU；`metal` feature 走 Apple GPU（`device`）。

mod device;
mod error;
mod hanzhang;

pub use error::NeuralError;
pub use hanzhang::{
    Candidate, CharLm, CharScorer, ModelConfig, P2c, P2cScorer, PrefixCache, Vocab, find_model, qjm,
};

#[cfg(feature = "zhiyi")]
pub mod zhiyi;
