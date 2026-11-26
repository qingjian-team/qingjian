//! 编码器与头共用的加性注意力掩码：前缀掩码（feather 起）与滑窗。

use candle_core::{Device, Result, Tensor};

/// 滑窗层只看 |i − j| ≤ 这么多的位置。
const WINDOW: usize = 64;

/// 屏蔽位置加的值。
const MASKED: f32 = -1.0e9;

/// 一条序列的两种掩码，`[t, t]`。
pub(super) struct Masks {
    /// 全局层与头用；没有前缀掩码时为 `None`（什么都不屏蔽）。
    pub global: Option<Tensor>,

    /// 滑窗层用（含前缀掩码）。
    pub sliding: Tensor,
}

impl Masks {
    /// `prefix` 是开头那一段的长度（第一个 `<eos>` 的位置加一）：段内的查询只看段内，段后的查询看全部。
    /// 模型没有前缀掩码时给 `None`。
    pub fn new(t: usize, prefix: Option<usize>, device: &Device) -> Result<Self> {
        let blocked = |i: usize, j: usize| prefix.is_some_and(|n| i < n && j >= n);
        let build = |window: bool| -> Result<Tensor> {
            let mask: Vec<f32> = (0..t)
                .flat_map(|i| {
                    (0..t).map(move |j| {
                        if blocked(i, j) || (window && i.abs_diff(j) > WINDOW) {
                            MASKED
                        } else {
                            0.0
                        }
                    })
                })
                .collect();
            Tensor::from_vec(mask, (t, t), device)
        };
        Ok(Self {
            global: prefix.map(|_| build(false)).transpose()?,
            sliding: build(true)?,
        })
    }
}
