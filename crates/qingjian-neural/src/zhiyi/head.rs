//! 选词头（与 feather 的整句读出），接在编码器后面。

use candle_core::{Result, Tensor};
use candle_nn::ops::softmax_last_dim;

use super::weights::{QLinear, Weights, layer_norm};

const HEAD_DIM: usize = 64;

/// PyTorch `nn.TransformerEncoderLayer`（norm_first、ReLU、无位置编码）。
struct HeadLayer {
    norm1: (Tensor, Tensor),
    in_proj: QLinear,
    out_proj: QLinear,
    norm2: (Tensor, Tensor),
    linear1: QLinear,
    linear2: QLinear,
    width: usize,
}

/// 选词头：几层 Transformer（L12 两层，feather 一层），再在每个候选标记处打一个分。
/// feather 的头与整句共用：同一份输出在每个音节槽位过 `out` 再与字向量点积，得各字的 logits。
pub(super) struct ChoiceHead {
    type_emb: Tensor,
    layers: Vec<HeadLayer>,
    scorer_norm: (Tensor, Tensor),
    scorer_hidden: QLinear,
    scorer_out: (Tensor, Tensor),

    /// 共用头的整句读出；L12 的整句在另一个头上，这里不接。
    compose: Option<ComposeReadout>,
}

/// `char_logits = out.1(out.0(h)) · char_embᵀ + char_bias`。
struct ComposeReadout {
    norm: (Tensor, Tensor),
    linear: QLinear,

    /// `[字数, 宽度]`，F32。
    char_emb: Tensor,

    char_bias: Tensor,
}

impl ChoiceHead {
    /// `shared` 为真时加载整句读出（`config.json` 的 `shared_head`）。
    pub fn load(w: &Weights, layer_count: usize, width: usize, shared: bool) -> Result<Self> {
        let norm = |name: &str| -> Result<(Tensor, Tensor)> {
            Ok((
                w.get(&format!("{name}.weight"))?,
                w.get(&format!("{name}.bias"))?,
            ))
        };
        let mut layers = Vec::with_capacity(layer_count);
        for i in 0..layer_count {
            let p = format!("head.layers.{i}");
            layers.push(HeadLayer {
                norm1: norm(&format!("{p}.norm1"))?,
                in_proj: w.linear(
                    &format!("{p}.self_attn.in_proj_weight"),
                    Some(&format!("{p}.self_attn.in_proj_bias")),
                )?,
                out_proj: w.linear(
                    &format!("{p}.self_attn.out_proj.weight"),
                    Some(&format!("{p}.self_attn.out_proj.bias")),
                )?,
                norm2: norm(&format!("{p}.norm2"))?,
                linear1: w.linear(
                    &format!("{p}.linear1.weight"),
                    Some(&format!("{p}.linear1.bias")),
                )?,
                linear2: w.linear(
                    &format!("{p}.linear2.weight"),
                    Some(&format!("{p}.linear2.bias")),
                )?,
                width,
            });
        }
        let compose = shared
            .then(|| -> Result<ComposeReadout> {
                Ok(ComposeReadout {
                    norm: norm("out.0")?,
                    linear: w.linear("out.1.weight", Some("out.1.bias"))?,
                    char_emb: w.get("char_emb.weight")?,
                    char_bias: w.get("char_bias")?,
                })
            })
            .transpose()?;
        Ok(Self {
            type_emb: w.get("type_emb.weight")?.narrow(0, 0, 1)?,
            layers,
            scorer_norm: norm("scorer.0")?,
            scorer_hidden: w.linear("scorer.1.weight", Some("scorer.1.bias"))?,
            scorer_out: norm("scorer.3")?,
            compose,
        })
    }

    /// 编码器输出 `[T, 宽度]` 过头的几层；`mask` 是前缀掩码（没有为 `None`）。
    pub fn hidden(&self, encoded: &Tensor, mask: Option<&Tensor>) -> Result<Tensor> {
        let mut h = encoded.broadcast_add(&self.type_emb)?;
        for layer in &self.layers {
            h = layer.forward(&h, mask)?;
        }
        Ok(h)
    }

    /// 头的输出 → 每个标记位置一个分（越大越好）。
    pub fn scores(&self, h: &Tensor, markers: &[u32]) -> Result<Vec<f32>> {
        let markers = Tensor::new(markers, h.device())?;
        let picked = h.index_select(&markers, 0)?;
        let x = layer_norm(&picked, &self.scorer_norm.0, Some(&self.scorer_norm.1))?;
        let x = self.scorer_hidden.forward(&x)?.gelu_erf()?;
        let (w, b) = &self.scorer_out;
        let score = x.matmul(&w.t()?)?.broadcast_add(b)?;
        score.squeeze(1)?.to_vec1()
    }

    /// 头的输出 → 每个槽位一行各字的 logits；没有整句读出时为 `None`。
    pub fn char_logits(&self, h: &Tensor, slots: &[u32]) -> Result<Option<Vec<Vec<f32>>>> {
        let Some(compose) = &self.compose else {
            return Ok(None);
        };
        let slots = Tensor::new(slots, h.device())?;
        let picked = h.index_select(&slots, 0)?;
        let q = compose.linear.forward(&layer_norm(
            &picked,
            &compose.norm.0,
            Some(&compose.norm.1),
        )?)?;
        let logits = q
            .matmul(&compose.char_emb.t()?)?
            .broadcast_add(&compose.char_bias)?;
        logits.to_vec2().map(Some)
    }
}

impl HeadLayer {
    fn forward(&self, x: &Tensor, mask: Option<&Tensor>) -> Result<Tensor> {
        let t = x.dim(0)?;
        let width = self.width;
        let h = layer_norm(x, &self.norm1.0, Some(&self.norm1.1))?;
        let qkv = self.in_proj.forward(&h)?;
        let heads = |i: usize| -> Result<Tensor> {
            qkv.narrow(1, i * width, width)?
                .reshape((t, width / HEAD_DIM, HEAD_DIM))?
                .transpose(0, 1)?
                .contiguous()
        };
        let (q, k, v) = (heads(0)?, heads(1)?, heads(2)?);
        let mut att = (q.matmul(&k.t()?)? * (1.0 / (HEAD_DIM as f64).sqrt()))?;
        if let Some(mask) = mask {
            att = att.broadcast_add(mask)?;
        }
        let out = softmax_last_dim(&att)?
            .matmul(&v)?
            .transpose(0, 1)?
            .reshape((t, width))?;
        let x = (x + self.out_proj.forward(&out)?)?;
        let h = layer_norm(&x, &self.norm2.0, Some(&self.norm2.1))?;
        x + self.linear2.forward(&self.linear1.forward(&h)?.relu()?)?
    }
}
