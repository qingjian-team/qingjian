use candle_core::{Result, Tensor};
use candle_nn::ops::softmax_last_dim;

use super::weights::{QLinear, Weights, layer_norm};

const WIDTH: usize = 384;
const HEADS: usize = 6;
const HEAD_DIM: usize = 64;

/// PyTorch `nn.TransformerEncoderLayer`（norm_first、ReLU、无位置编码）。
struct HeadLayer {
    norm1: (Tensor, Tensor),
    in_proj: QLinear,
    out_proj: QLinear,
    norm2: (Tensor, Tensor),
    linear1: QLinear,
    linear2: QLinear,
}

/// 选词头：两层 Transformer，再在每个候选标记处打一个分。
pub(super) struct ChoiceHead {
    type_emb: Tensor,
    layers: Vec<HeadLayer>,
    scorer_norm: (Tensor, Tensor),
    scorer_hidden: QLinear,
    scorer_out: (Tensor, Tensor),
}

impl ChoiceHead {
    pub fn load(w: &Weights) -> Result<Self> {
        let norm = |name: &str| -> Result<(Tensor, Tensor)> {
            Ok((
                w.get(&format!("{name}.weight"))?,
                w.get(&format!("{name}.bias"))?,
            ))
        };
        let mut layers = Vec::new();
        for i in 0..2 {
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
            });
        }
        Ok(Self {
            type_emb: w.get("type_emb.weight")?.narrow(0, 0, 1)?,
            layers,
            scorer_norm: norm("scorer.0")?,
            scorer_hidden: w.linear("scorer.1.weight", Some("scorer.1.bias"))?,
            scorer_out: norm("scorer.3")?,
        })
    }

    /// 编码器输出 `[T, 384]` → 每个标记位置一个分（越大越好）。
    pub fn scores(&self, encoded: &Tensor, markers: &[u32]) -> Result<Vec<f32>> {
        let mut h = encoded.broadcast_add(&self.type_emb)?;
        for layer in &self.layers {
            h = layer.forward(&h)?;
        }
        let markers = Tensor::new(markers, h.device())?;
        let picked = h.index_select(&markers, 0)?;
        let x = layer_norm(&picked, &self.scorer_norm.0, Some(&self.scorer_norm.1))?;
        let x = self.scorer_hidden.forward(&x)?.gelu_erf()?;
        let (w, b) = &self.scorer_out;
        let score = x.matmul(&w.t()?)?.broadcast_add(b)?;
        score.squeeze(1)?.to_vec1()
    }
}

impl HeadLayer {
    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let t = x.dim(0)?;
        let h = layer_norm(x, &self.norm1.0, Some(&self.norm1.1))?;
        let qkv = self.in_proj.forward(&h)?;
        let heads = |i: usize| -> Result<Tensor> {
            qkv.narrow(1, i * WIDTH, WIDTH)?
                .reshape((t, HEADS, HEAD_DIM))?
                .transpose(0, 1)?
                .contiguous()
        };
        let (q, k, v) = (heads(0)?, heads(1)?, heads(2)?);
        let att = (q.matmul(&k.t()?)? * (1.0 / (HEAD_DIM as f64).sqrt()))?;
        let out = softmax_last_dim(&att)?
            .matmul(&v)?
            .transpose(0, 1)?
            .reshape((t, WIDTH))?;
        let x = (x + self.out_proj.forward(&out)?)?;
        let h = layer_norm(&x, &self.norm2.0, Some(&self.norm2.1))?;
        x + self.linear2.forward(&self.linear1.forward(&h)?.relu()?)?
    }
}
