//! 知意的编码器（ModernBERT 层），safetensors 版用。

use candle_core::{D, Device, Result, Tensor};
use candle_nn::ops::softmax_last_dim;

use super::mask::Masks;
use super::weights::{QLinear, Weights, layer_norm};

const HEAD_DIM: usize = 64;
const ROPE_BASE: f64 = 160_000.0;

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
    width: usize,
    intermediate: usize,
}

/// mmBERT-small 剪成的编码器（L12 留 12 层宽 384，feather 8 层宽 256），层数与宽度按 `config.json`。
pub(super) struct Encoder {
    tok_emb: Tensor,
    /// 分词器 id → 嵌入行。在 CPU 上查表，不必上 GPU。
    vocab_remap: Vec<u32>,
    unit_emb: Tensor,

    /// 注音声调（feather 起）：单元号 = 单元 + 单元表行数 × 声调，第 0 行（无声调）全零；L12 没有。
    tone_emb: Option<Tensor>,

    /// 加在每个候选第一个 token 上（紧凑格式）；旧格式没有。
    cand_emb: Option<Tensor>,

    emb_norm: Tensor,
    layers: Vec<Layer>,
    final_norm: Tensor,
    device: Device,
}

impl Encoder {
    pub fn load(
        w: &Weights,
        sliding: &[bool],
        (width, intermediate): (usize, usize),
        device: &Device,
    ) -> Result<Self> {
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
                width,
                intermediate,
            });
        }
        Ok(Self {
            tok_emb: w.raw("encoder.embeddings.tok_embeddings.weight")?,
            vocab_remap: w.table("vocab_remap")?,
            unit_emb: w.get("unit_emb.weight")?,
            tone_emb: w
                .has("tone_emb.weight")
                .then(|| w.get("tone_emb.weight"))
                .transpose()?,
            cand_emb: w.has("cand_emb").then(|| w.get("cand_emb")).transpose()?,
            emb_norm: w.get("encoder.embeddings.norm.weight")?,
            layers,
            final_norm: w.get("encoder.final_norm.weight")?,
            device: device.clone(),
        })
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    /// `ids` 是分词器的原始 id（编码前先经 `vocab_remap` 映射到嵌入行），`units` 是每个位置的拼音单元（0 为无），
    /// `markers` 是候选标记位置（有 `cand_emb` 时加在那里）。返回 `[T, 宽度]`。
    pub fn forward(
        &self,
        ids: &[u32],
        units: &[u32],
        markers: &[u32],
        masks: &Masks,
    ) -> Result<Tensor> {
        let t = ids.len();
        let rows: Vec<u32> = ids
            .iter()
            .map(|&id| self.vocab_remap.get(id as usize).copied().unwrap_or(0))
            .collect();
        let rows = Tensor::new(rows, &self.device)?;
        let tokens = self
            .tok_emb
            .index_select(&rows, 0)?
            .to_dtype(candle_core::DType::F32)?;
        let mut x = match &self.tone_emb {
            Some(tone_emb) => {
                let stride = self.unit_emb.dim(0)? as u32;
                let plain: Vec<u32> = units.iter().map(|&u| u % stride).collect();
                let tones: Vec<u32> = units.iter().map(|&u| u / stride).collect();
                let plain = self
                    .unit_emb
                    .index_select(&Tensor::new(plain, &self.device)?, 0)?;
                let tones = tone_emb.index_select(&Tensor::new(tones, &self.device)?, 0)?;
                ((tokens + plain)? + tones)?
            }
            None => {
                (tokens
                    + self
                        .unit_emb
                        .index_select(&Tensor::new(units, &self.device)?, 0)?)?
            }
        };
        if let Some(cand_emb) = &self.cand_emb {
            let mut at = vec![0f32; t];
            for &m in markers {
                if let Some(slot) = at.get_mut(m as usize) {
                    *slot = 1.0;
                }
            }
            let at = Tensor::from_vec(at, (t, 1), &self.device)?;
            x = (x + at.broadcast_mul(&cand_emb.unsqueeze(0)?)?)?;
        }
        let mut x = layer_norm(&x, &self.emb_norm, None)?;
        let (cos, sin) = rope(t, &self.device)?;
        for layer in &self.layers {
            let mask = if layer.sliding {
                Some(&masks.sliding)
            } else {
                masks.global.as_ref()
            };
            x = layer.forward(&x, &cos, &sin, mask)?;
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
        let heads = self.width / HEAD_DIM;
        let h = match &self.attn_norm {
            Some(norm) => layer_norm(x, norm, None)?,
            None => x.clone(),
        };
        let qkv = self.wqkv.forward(&h)?.reshape((t, 3, heads, HEAD_DIM))?;
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
            .reshape((t, self.width))?;
        let x = (x + self.wo.forward(&out)?)?;
        let m = self.wi.forward(&layer_norm(&x, &self.mlp_norm, None)?)?;
        let a = m.narrow(D::Minus1, 0, self.intermediate)?;
        let g = m.narrow(D::Minus1, self.intermediate, self.intermediate)?;
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
