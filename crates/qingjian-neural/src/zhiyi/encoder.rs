use candle_core::{D, Device, Result, Tensor};
use candle_nn::ops::softmax_last_dim;

use super::weights::{QLinear, Weights, layer_norm};

const WIDTH: usize = 384;
const HEADS: usize = 6;
const HEAD_DIM: usize = 64;
const INTERMEDIATE: usize = 1152;
const ROPE_BASE: f64 = 160_000.0;

/// 滑窗层只看 |i − j| ≤ 这么多的位置。
const WINDOW: usize = 64;

/// 屏蔽位置加的值。
const MASKED: f32 = -1.0e9;

/// 一层 ModernBERT：pre-norm 注意力（RoPE，全局或滑窗）+ GeGLU MLP，都没有偏置。
struct Layer {
    /// 第 0 层没有注意力前的范数。
    attn_norm: Option<Tensor>,
    wqkv: QLinear,
    wo: QLinear,
    mlp_norm: Tensor,
    wi: QLinear,
    mlp_wo: QLinear,
    sliding: bool,
}

/// mmBERT-small 留下的 12 层编码器。
pub(super) struct Encoder {
    tok_emb: Tensor,
    /// 分词器 id → 嵌入行。在 CPU 上查表，不必上 GPU。
    vocab_remap: Vec<u32>,
    unit_emb: Tensor,
    emb_norm: Tensor,
    layers: Vec<Layer>,
    final_norm: Tensor,
    device: Device,
}

impl Encoder {
    pub fn load(w: &Weights, sliding: &[bool], device: &Device) -> Result<Self> {
        let mut layers = Vec::with_capacity(sliding.len());
        for (i, &sliding) in sliding.iter().enumerate() {
            let p = format!("encoder.layers.{i}");
            layers.push(Layer {
                attn_norm: (i > 0)
                    .then(|| w.get(&format!("{p}.attn_norm.weight")))
                    .transpose()?,
                wqkv: w.linear(&format!("{p}.attn.Wqkv.weight"), None)?,
                wo: w.linear(&format!("{p}.attn.Wo.weight"), None)?,
                mlp_norm: w.get(&format!("{p}.mlp_norm.weight"))?,
                wi: w.linear(&format!("{p}.mlp.Wi.weight"), None)?,
                mlp_wo: w.linear(&format!("{p}.mlp.Wo.weight"), None)?,
                sliding,
            });
        }
        Ok(Self {
            tok_emb: w.raw("encoder.embeddings.tok_embeddings.weight")?,
            vocab_remap: w.table("vocab_remap")?,
            unit_emb: w.get("unit_emb.weight")?,
            emb_norm: w.get("encoder.embeddings.norm.weight")?,
            layers,
            final_norm: w.get("encoder.final_norm.weight")?,
            device: device.clone(),
        })
    }

    /// `ids` 是分词器的原始 id（编码前先经 `vocab_remap` 映射到嵌入行），`units` 是每个位置的拼音单元（0 为无）。
    /// 返回 `[T, 384]`。
    pub fn forward(&self, ids: &[u32], units: &[u32]) -> Result<Tensor> {
        let t = ids.len();
        let rows: Vec<u32> = ids
            .iter()
            .map(|&id| self.vocab_remap.get(id as usize).copied().unwrap_or(0))
            .collect();
        let rows = Tensor::new(rows, &self.device)?;
        let units = Tensor::new(units, &self.device)?;
        let tokens = self
            .tok_emb
            .index_select(&rows, 0)?
            .to_dtype(candle_core::DType::F32)?;
        let x = (tokens + self.unit_emb.index_select(&units, 0)?)?;
        let mut x = layer_norm(&x, &self.emb_norm, None)?;
        let (cos, sin) = rope(t, &self.device)?;
        let window = window_mask(t, &self.device)?;
        for layer in &self.layers {
            x = layer.forward(&x, &cos, &sin, layer.sliding.then_some(&window))?;
        }
        layer_norm(&x, &self.final_norm, None)
    }
}

impl Layer {
    fn forward(
        &self,
        x: &Tensor,
        cos: &Tensor,
        sin: &Tensor,
        mask: Option<&Tensor>,
    ) -> Result<Tensor> {
        let t = x.dim(0)?;
        let h = match &self.attn_norm {
            Some(norm) => layer_norm(x, norm, None)?,
            None => x.clone(),
        };
        let qkv = self.wqkv.forward(&h)?.reshape((t, 3, HEADS, HEAD_DIM))?;
        let part = |i: usize| -> Result<Tensor> {
            qkv.narrow(1, i, 1)?
                .squeeze(1)?
                .transpose(0, 1)?
                .contiguous()
        };
        let q = apply_rope(&part(0)?, cos, sin)?;
        let k = apply_rope(&part(1)?, cos, sin)?;
        let v = part(2)?;
        let mut att = (q.matmul(&k.t()?)? * (1.0 / (HEAD_DIM as f64).sqrt()))?;
        if let Some(mask) = mask {
            att = att.broadcast_add(mask)?;
        }
        let out = softmax_last_dim(&att)?
            .matmul(&v)?
            .transpose(0, 1)?
            .reshape((t, WIDTH))?;
        let x = (x + self.wo.forward(&out)?)?;
        let m = self.wi.forward(&layer_norm(&x, &self.mlp_norm, None)?)?;
        let a = m.narrow(D::Minus1, 0, INTERMEDIATE)?;
        let g = m.narrow(D::Minus1, INTERMEDIATE, INTERMEDIATE)?;
        x + self.mlp_wo.forward(&(a.gelu_erf()? * g)?)?
    }
}

/// 位置 0…t−1 的 cos / sin，`[t, 64]`（前后两半同一组频率，配 rotate-half）。
fn rope(t: usize, device: &Device) -> Result<(Tensor, Tensor)> {
    let half = HEAD_DIM / 2;
    let mut angles = Vec::with_capacity(t * HEAD_DIM);
    for pos in 0..t {
        for i in 0..HEAD_DIM {
            let freq = 1.0 / ROPE_BASE.powf((2 * (i % half)) as f64 / HEAD_DIM as f64);
            angles.push((pos as f64 * freq) as f32);
        }
    }
    let angles = Tensor::from_vec(angles, (t, HEAD_DIM), device)?;
    Ok((angles.cos()?, angles.sin()?))
}

/// `x·cos + rotate_half(x)·sin`，`x` 是 `[heads, t, 64]`。
fn apply_rope(x: &Tensor, cos: &Tensor, sin: &Tensor) -> Result<Tensor> {
    let half = HEAD_DIM / 2;
    let x1 = x.narrow(D::Minus1, 0, half)?;
    let x2 = x.narrow(D::Minus1, half, half)?;
    let rotated = Tensor::cat(&[&x2.neg()?, &x1], D::Minus1)?;
    x.broadcast_mul(cos)? + rotated.broadcast_mul(sin)?
}

/// 滑窗层的加性掩码 `[t, t]`：|i − j| ≤ 64 为 0，其余屏蔽。
fn window_mask(t: usize, device: &Device) -> Result<Tensor> {
    let mask: Vec<f32> = (0..t)
        .flat_map(|i| (0..t).map(move |j| if i.abs_diff(j) <= WINDOW { 0.0 } else { MASKED }))
        .collect();
    Tensor::from_vec(mask, (t, t), device)
}
