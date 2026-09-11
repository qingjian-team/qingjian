use candle_core::{D, DType, Device, Module, Result, Tensor};
use candle_nn::{Embedding, LayerNorm, Linear, VarBuilder, layer_norm, linear, ops};

use crate::ModelConfig;

/// 一层：pre-LN 自注意力 + pre-LN MLP，都带残差。
struct Block {
    ln1: LayerNorm,
    qkv: Linear,
    attn_proj: Linear,
    ln2: LayerNorm,
    fc: Linear,
    mlp_proj: Linear,
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
        let head = Linear::new(tok_weight, None);
        Ok(Self {
            tok_emb,
            pos_emb,
            blocks,
            ln_f,
            head,
            cfg,
            device,
        })
    }

    pub fn config(&self) -> &ModelConfig {
        &self.cfg
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    /// 因果掩码：上三角（未来位置）加上极小值。
    fn causal_mask(&self, t: usize) -> Result<Tensor> {
        let data: Vec<f32> = (0..t)
            .flat_map(|i| (0..t).map(move |j| if j > i { f32::MIN } else { 0.0 }))
            .collect();
        Tensor::from_vec(data, (t, t), &self.device)
    }

    fn attention(&self, block: &Block, x: &Tensor, mask: &Tensor) -> Result<Tensor> {
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
            .transpose(1, 2)?;
        let v = qkv
            .narrow(2, 2 * c, c)?
            .reshape((b, t, h, d))?
            .transpose(1, 2)?;
        let scale = 1.0 / (d as f64).sqrt();
        let att = (q.matmul(&k.transpose(2, 3)?.contiguous()?)? * scale)?;
        let att = att.broadcast_add(mask)?;
        let att = ops::softmax_last_dim(&att)?;
        let y = att.matmul(&v.contiguous()?)?;
        let y = y.transpose(1, 2)?.contiguous()?.reshape((b, t, c))?;
        block.attn_proj.forward(&y)
    }

    /// 前向：`idx` 形状 `[b, t]`（u32），返回 logits `[b, t, vocab]`。
    pub fn forward(&self, idx: &Tensor) -> Result<Tensor> {
        let (_, t) = idx.dims2()?;
        let mask = self.causal_mask(t)?;
        let pos = self.pos_emb.narrow(0, 0, t)?;
        let mut x = self.tok_emb.forward(idx)?.broadcast_add(&pos)?;
        for block in &self.blocks {
            let a = self.attention(block, &block.ln1.forward(&x)?, &mask)?;
            x = (x + a)?;
            let m = block
                .mlp_proj
                .forward(&block.fc.forward(&block.ln2.forward(&x)?)?.gelu_erf()?)?;
            x = (x + m)?;
        }
        let x = self.ln_f.forward(&x)?;
        self.head.forward(&x)
    }

    /// 每个位置对下一个 token 的 log-softmax，`[b, t, vocab]`，f32。
    pub fn log_probs(&self, idx: &Tensor) -> Result<Tensor> {
        let logits = self.forward(idx)?.to_dtype(DType::F32)?;
        ops::log_softmax(&logits, D::Minus1)
    }
}
