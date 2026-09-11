use candle_core::{D, DType, Device, Module, Result, Tensor};
use candle_nn::{Embedding, LayerNorm, Linear, VarBuilder, layer_norm, linear, ops};

use crate::ModelConfig;

/// 掩码里未来位置加的值：够大到 softmax 后为 0，又在 f16 范围内。
const MASKED: f32 = -1.0e4;

/// 一层：pre-LN 自注意力 + pre-LN MLP，都带残差。
struct Block {
    ln1: LayerNorm,
    qkv: Linear,
    attn_proj: Linear,
    ln2: LayerNorm,
    fc: Linear,
    mlp_proj: Linear,
}

/// 一段前文在每层的 K / V（形状 `[1, h, p, d]`）。前文在一次组句里不变，算一次存下来，
/// 候选接在后面时只算候选自己那几个 token（见 [`CharLm::log_probs_after`]）。
pub struct PrefixCache {
    keys: Vec<Tensor>,
    values: Vec<Tensor>,
    len: usize,
}

impl PrefixCache {
    /// 前文的 token 数。
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// 字级 decoder-only Transformer。张量名见 `tools/lm-train/model.py`。
pub struct CharLm {
    tok_emb: Embedding,
    pos_emb: Tensor,
    blocks: Vec<Block>,
    ln_f: LayerNorm,
    /// 输出层复用输入嵌入。
    head: Linear,
    cfg: ModelConfig,
    device: Device,
    /// 权重与中间量的精度（f32，或 Metal 上的 f16）。
    dtype: DType,
}

impl CharLm {
    pub fn load(vb: VarBuilder, cfg: ModelConfig, device: Device) -> Result<Self> {
        let tok_weight = vb.get((cfg.vocab_size, cfg.n_embd), "tok_emb.weight")?;
        let tok_emb = Embedding::new(tok_weight.clone(), cfg.n_embd);
        let pos_emb = vb.get((cfg.context, cfg.n_embd), "pos_emb.weight")?;
        let mut blocks = Vec::with_capacity(cfg.n_layer);
        for i in 0..cfg.n_layer {
            let b = vb.pp(format!("blocks.{i}"));
            blocks.push(Block {
                ln1: layer_norm(cfg.n_embd, 1e-5, b.pp("ln1"))?,
                qkv: linear(cfg.n_embd, 3 * cfg.n_embd, b.pp("attn.qkv"))?,
                attn_proj: linear(cfg.n_embd, cfg.n_embd, b.pp("attn.proj"))?,
                ln2: layer_norm(cfg.n_embd, 1e-5, b.pp("ln2"))?,
                fc: linear(cfg.n_embd, 4 * cfg.n_embd, b.pp("mlp.fc"))?,
                mlp_proj: linear(4 * cfg.n_embd, cfg.n_embd, b.pp("mlp.proj"))?,
            });
        }
        let ln_f = layer_norm(cfg.n_embd, 1e-5, vb.pp("ln_f"))?;
        let dtype = tok_weight.dtype();
        let head = Linear::new(tok_weight, None);
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

    /// 一层注意力。`past` 是前文的 K / V（`[1, h, p, d]`），有就拼在本段 K / V 前面。
    /// 返回输出与本段自己的 K / V（`[b, h, t, d]`），记前文缓存用。
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
            None => (k.clone(), v.clone()),
        };
        let scale = 1.0 / (d as f64).sqrt();
        let att = (q.matmul(&k_all.transpose(2, 3)?.contiguous()?)? * scale)?;
        let att = att.broadcast_add(mask)?;
        let att = ops::softmax_last_dim(&att)?;
        let y = att.matmul(&v_all)?;
        let y = y.transpose(1, 2)?.contiguous()?.reshape((b, t, c))?;
        Ok((block.attn_proj.forward(&y)?, k, v))
    }

    /// 跑一段 token：`idx` 形状 `[b, t]`，位置从 `past` 的长度接着数；`record` 为真时把每层的 K / V 收成缓存返回
    /// （只在算前文时用，此时 `b` 是 1）。返回 logits `[b, t, vocab]`。
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
        let mut x = self.tok_emb.forward(idx)?.broadcast_add(&pos)?;
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
        Ok(self.run(idx, None, false)?.0)
    }

    /// 每个位置对下一个 token 的 log-softmax，`[b, t, vocab]`，f32。
    pub fn log_probs(&self, idx: &Tensor) -> Result<Tensor> {
        let logits = self.forward(idx)?.to_dtype(DType::F32)?;
        ops::log_softmax(&logits, D::Minus1)
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
        let (_, cache) = self.run(&idx, None, true)?;
        Ok(cache.expect("record was requested"))
    }

    /// 接在前文缓存后面的一段（`[b, t]`）每个位置对下一个 token 的 log-softmax，`[b, t, vocab]`，f32。
    /// 前文长度加 `t` 不能超过模型上下文。
    pub fn log_probs_after(&self, cache: &PrefixCache, idx: &Tensor) -> Result<Tensor> {
        let (logits, _) = self.run(idx, Some(cache), false)?;
        ops::log_softmax(&logits.to_dtype(DType::F32)?, D::Minus1)
    }
}
