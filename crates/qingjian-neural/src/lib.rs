//! 本地神经模型的推理（candle，知意的 ONNX 版走 ONNX Runtime），两套模型各一个目录：
//!
//! - `hanzhang`：含章，自训的字级 Transformer。给「光标前文 + 候选」打分（[`CharScorer`]），或按拼音逐字生成整句（[`P2c`]）。
//! - `zhiyi`：知意的选词模型（feature `zhiyi`）。给候选重排，并按音节出字。
//!
//! 缺省 CPU；`metal` feature 走 Apple GPU（`device`）。

mod device;
mod error;
mod hanzhang;

use std::path::Path;

pub use error::NeuralError;
pub use hanzhang::{
    Candidate, CharLm, CharScorer, ModelConfig, P2c, P2cScorer, PrefixCache, Vocab, find_model, qjm,
};

/// 模型目录里文本文件（`vocab.json`、`config.json`、`merges.json`）的字节上限：
/// 中文词表在十 MB 量级，模型目录可能是从外面下载的整包，读之前先挡一道。
pub(crate) const MAX_TEXT_BYTES: u64 = 32 << 20;

/// 读模型目录里的文本文件，超过 [`MAX_TEXT_BYTES`] 的报 [`NeuralError::TooLarge`]。
pub(crate) fn read_text_limited(path: &Path) -> Result<String, NeuralError> {
    let too_large = |bytes| NeuralError::TooLarge {
        path: path.to_owned(),
        bytes,
    };
    let size = std::fs::metadata(path)
        .map_err(|source| NeuralError::Io {
            path: path.to_owned(),
            source,
        })?
        .len();
    if size > MAX_TEXT_BYTES {
        return Err(too_large(size));
    }
    let text = std::fs::read_to_string(path).map_err(|source| NeuralError::Io {
        path: path.to_owned(),
        source,
    })?;
    if text.len() as u64 > MAX_TEXT_BYTES {
        return Err(too_large(text.len() as u64));
    }
    Ok(text)
}

#[cfg(feature = "zhiyi")]
pub mod zhiyi;
