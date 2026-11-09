//! 字级 decoder-only Transformer（[`CharLm`]）的前向与前文缓存。

mod prefix_cache;
mod projection;
mod q8_matrix;
mod threads;

pub use prefix_cache::PrefixCache;

use candle_core::{D, DType, Device, Module, Result, Tensor};
use candle_nn::{Embedding, LayerNorm, VarBuilder, layer_norm, ops};

use super::ModelConfig;
use projection::Projection;

pub(crate) use q8_matrix::Q8Matrix;

/// 掩码里未来位置加的值：够大到 softmax 后为 0，又在 f16 范围内。
const MASKED: f32 = -1.0e4;

/// 一层：pre-LN 自注意力 + pre-LN MLP，都带残差。
struct Block {
    ln1: LayerNorm,

    qkv: Projection,

    attn_proj: Projection,

    ln2: LayerNorm,

    fc: Projection,

    mlp_proj: Projection,
}

/// 字级 decoder-only Transformer。张量名见 `tools/lm-train/model.py`。
pub struct CharLm {
    /// 量化时留文件里的 f16（每次只查几十行），查完转成 `dtype`。
    tok_emb: Embedding,

    pos_emb: Tensor,

    blocks: Vec<Block>,

    ln_f: LayerNorm,

    /// 输出层复用输入嵌入。
    head: Projection,

    cfg: ModelConfig,

    device: Device,

    /// 中间量的精度（f32，或 Metal 上的 f16），也是不量化时权重的精度。
    dtype: DType,
}

impl CharLm {
    /// `quantize` 为真时线性层与输出层量化成 Q8_0、词嵌入留 f16，其余按 `vb` 的精度（应为 f32）：
    /// CPU 上权重占的内存约为 f32 的四分之一。
    pub fn load(vb: VarBuilder, cfg: ModelConfig, device: Device, quantize: bool) -> Result<Self> {
        let dtype = vb.dtype();
        let embedding = if quantize {
            vb.to_dtype(DType::F16)
        } else {
            vb.clone()
        };
        let tok_weight = embedding.get((cfg.vocab_size, cfg.n_embd), "tok_emb.weight")?;
        let tok_emb = Embedding::new(tok_weight.clone(), cfg.n_embd);
        let pos_emb = vb.get((cfg.context, cfg.n_embd), "pos_emb.weight")?;
        let c = cfg.n_embd;
        let mut blocks = Vec::with_capacity(cfg.n_layer);
        for i in 0..cfg.n_layer {
            let b = vb.pp(format!("blocks.{i}"));
            blocks.push(Block {
                ln1: layer_norm(c, 1e-5, b.pp("ln1"))?,
                qkv: Projection::load(b.pp("attn.qkv"), c, 3 * c, quantize)?,
                attn_proj: Projection::load(b.pp("attn.proj"), c, c, quantize)?,
                ln2: layer_norm(c, 1e-5, b.pp("ln2"))?,
                fc: Projection::load(b.pp("mlp.fc"), c, 4 * c, quantize)?,
                mlp_proj: Projection::load(b.pp("mlp.proj"), 4 * c, c, quantize)?,
            });
        }
        let ln_f = layer_norm(c, 1e-5, vb.pp("ln_f"))?;
        let head = Projection::tied(tok_weight, quantize)?;
        Ok(Self {
            tok_emb,
            pos_emb,
            blocks,
            ln_f,
            head,
            cfg,
            device,
            dtype,
        })
    }

    pub fn config(&self) -> &ModelConfig {
        &self.cfg
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    /// 因果掩码 `[t, past + t]`：前 `past` 列是前文，全部可见；后面 `t` 列里未来位置加上极小值。
    fn causal_mask(&self, t: usize, past: usize) -> Result<Tensor> {
        let width = past + t;
        let data: Vec<f32> = (0..t)
            .flat_map(|i| (0..width).map(move |j| if j > past + i { MASKED } else { 0.0 }))
            .collect();
        Tensor::from_vec(data, (t, width), &self.device)?.to_dtype(self.dtype)
    }

    /// 一层注意力。`past` 是前文的 K / V（`[1, h, p, d]` 或 `[b, h, p, d]`），有就拼在本段 K / V 前面。
    /// 返回输出与拼好的 K / V（`[b, h, p + t, d]`），记缓存用。
    fn attention(
        &self,
        block: &Block,
        x: &Tensor,
        mask: &Tensor,
        past: Option<(&Tensor, &Tensor)>,
    ) -> Result<(Tensor, Tensor, Tensor)> {
        let (b, t, c) = x.dims3()?;
        let h = self.cfg.n_head;
        let d = c / h;
        let qkv = block.qkv.forward(x)?;
        let q = qkv
            .narrow(2, 0, c)?
            .reshape((b, t, h, d))?
            .transpose(1, 2)?
            .contiguous()?;
        let k = qkv
            .narrow(2, c, c)?
            .reshape((b, t, h, d))?
            .transpose(1, 2)?
            .contiguous()?;
        let v = qkv
            .narrow(2, 2 * c, c)?
            .reshape((b, t, h, d))?
            .transpose(1, 2)?
            .contiguous()?;
        let (k_all, v_all) = match past {
            Some((pk, pv)) => {
                let p = pk.dim(2)?;
                let pk = pk.broadcast_as((b, h, p, d))?.contiguous()?;
                let pv = pv.broadcast_as((b, h, p, d))?.contiguous()?;
                (Tensor::cat(&[&pk, &k], 2)?, Tensor::cat(&[&pv, &v], 2)?)
            }
            None => (k, v),
        };
        let scale = 1.0 / (d as f64).sqrt();
        let att = (q.matmul(&k_all.transpose(2, 3)?.contiguous()?)? * scale)?;
        let att = att.broadcast_add(mask)?;
        let att = ops::softmax_last_dim(&att)?;
        let y = att.matmul(&v_all)?;
        let y = y.transpose(1, 2)?.contiguous()?.reshape((b, t, c))?;
        Ok((block.attn_proj.forward(&y)?, k_all, v_all))
    }

    /// 跑一段 token：`idx` 形状 `[b, t]`，位置从 `past` 的长度接着数；`record` 为真时把每层的 K / V
    /// 连同前文一起收成缓存返回。返回 logits `[b, t, vocab]`。
    fn run(
        &self,
        idx: &Tensor,
        past: Option<&PrefixCache>,
        record: bool,
    ) -> Result<(Tensor, Option<PrefixCache>)> {
        let (_, t) = idx.dims2()?;
        let offset = past.map_or(0, PrefixCache::len);
        let mask = self.causal_mask(t, offset)?;
        let pos = self.pos_emb.narrow(0, offset, t)?;
        let mut x = self
            .tok_emb
            .forward(idx)?
            .to_dtype(self.dtype)?
            .broadcast_add(&pos)?;
        let mut keys = Vec::new();
        let mut values = Vec::new();
        for (i, block) in self.blocks.iter().enumerate() {
            let layer_past = past
                .filter(|cache| !cache.is_empty())
                .map(|cache| (&cache.keys[i], &cache.values[i]));
            let (a, k, v) = self.attention(block, &block.ln1.forward(&x)?, &mask, layer_past)?;
            if record {
                keys.push(k);
                values.push(v);
            }
            x = (x + a)?;
            let m = block
                .mlp_proj
                .forward(&block.fc.forward(&block.ln2.forward(&x)?)?.gelu_erf()?)?;
            x = (x + m)?;
        }
        let x = self.ln_f.forward(&x)?;
        let logits = self.head.forward(&x)?;
        let cache = record.then_some(PrefixCache {
            keys,
            values,
            len: offset + t,
        });
        Ok((logits, cache))
    }

    /// 前向：`idx` 形状 `[b, t]`（u32），返回 logits `[b, t, vocab]`。
    pub fn forward(&self, idx: &Tensor) -> Result<Tensor> {
        threads::install(|| Ok(self.run(idx, None, false)?.0))
    }

    /// 每个位置对下一个 token 的 log-softmax，`[b, t, vocab]`，f32。
    pub fn log_probs(&self, idx: &Tensor) -> Result<Tensor> {
        threads::install(|| {
            let logits = self.run(idx, None, false)?.0.to_dtype(DType::F32)?;
            ops::log_softmax(&logits, D::Minus1)
        })
    }

    /// 算一段前文的 K / V 缓存；空前文给空缓存。
    pub fn prefix_cache(&self, ids: &[u32]) -> Result<PrefixCache> {
        if ids.is_empty() {
            return Ok(PrefixCache {
                keys: Vec::new(),
                values: Vec::new(),
                len: 0,
            });
        }
        let idx = Tensor::from_vec(ids.to_vec(), (1, ids.len()), &self.device)?;
        let (_, cache) = threads::install(|| self.run(&idx, None, true))?;
        Ok(cache.expect("record was requested"))
    }

    /// 接着缓存推进一段（`idx` 形状 `[b, t]`），返回最后一位对下一个 token 的 log-softmax（`[b, vocab]`，f32）
    /// 与长出这一段之后的新缓存。逐字生成用。
    pub fn step(&self, cache: &PrefixCache, idx: &Tensor) -> Result<(Tensor, PrefixCache)> {
        threads::install(|| {
            let (logits, next) = self.run(idx, Some(cache), true)?;
            let t = logits.dim(1)?;
            let last = logits
                .narrow(1, t - 1, 1)?
                .squeeze(1)?
                .to_dtype(DType::F32)?;
            Ok((
                ops::log_softmax(&last, D::Minus1)?,
                next.expect("record was requested"),
            ))
        })
    }

    /// 接在前文缓存后面的一段（`[b, t]`）每个位置对下一个 token 的 log-softmax，`[b, t, vocab]`，f32。
    /// 前文长度加 `t` 不能超过模型上下文。
    pub fn log_probs_after(&self, cache: &PrefixCache, idx: &Tensor) -> Result<Tensor> {
        threads::install(|| {
            let (logits, _) = self.run(idx, Some(cache), false)?;
            ops::log_softmax(&logits.to_dtype(DType::F32)?, D::Minus1)
        })
    }
}
