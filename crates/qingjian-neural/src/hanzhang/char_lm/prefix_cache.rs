//! 前文的 K / V 缓存。

use candle_core::{Result, Tensor};

/// 一段前文在每层的 K / V（形状 `[1, h, p, d]`）。前文在一次组句里不变，算一次存下来，
/// 候选接在后面时只算候选自己那几个 token（见 [`super::CharLm::log_probs_after`]）。
pub struct PrefixCache {
    pub(super) keys: Vec<Tensor>,

    pub(super) values: Vec<Tensor>,

    pub(super) len: usize,
}

impl PrefixCache {
    /// 前文的 token 数。
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// 按行号重排 batch 维：beam search 每步挑完候选要把各条的 K / V 跟着搬。
    /// `order` 是 `[b]` 的 u32 行号，允许重复（一行可以分裂成多条）。
    pub fn select(&self, order: &Tensor) -> Result<Self> {
        let pick = |t: &Tensor| t.index_select(order, 0);
        Ok(Self {
            keys: self.keys.iter().map(pick).collect::<Result<_>>()?,
            values: self.values.iter().map(pick).collect::<Result<_>>()?,
            len: self.len,
        })
    }
}
