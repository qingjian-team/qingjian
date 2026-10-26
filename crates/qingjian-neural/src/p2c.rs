//! 带噪拼音 → 汉字（beam search）。
//!
//! 训练序列是 `[带噪拼音] <sep> [汉字] <eos>`，所以解码就是：把拼音和 `<sep>` 喂进去当前缀，
//! 然后逐字生成到 `<eos>`。前缀的 K/V 算一次存下来，每敲一键只需把新字母接上去。
//!
//! 前缀开头要带一个 `<eos>`：训练流里每个样本的拼音前面都跟着上一个样本的 `<eos>`，
//! 少了它模型会当成在续写上文、把短拼音看成被截断的尾巴，于是把正确的词放句尾再补一堆前缀
//! （`jiekou` → 我们可以使用多个接口）。

use candle_core::Tensor;

use crate::vocab::{EOS, Vocab};
use crate::{CharLm, NeuralError};

/// 一条候选。
#[derive(Debug, Clone)]
pub struct Candidate {
    pub text: String,

    /// 生成这段汉字的 log 概率之和（越接近 0 越好）。
    pub score: f64,
}

/// P2C 解码器：借用已加载的模型与字表。
pub struct P2c<'a> {
    model: &'a CharLm,
    vocab: &'a Vocab,
    sep: u32,
}

impl<'a> P2c<'a> {
    /// 字表里没有 `<sep>`（是字级 LM 的模型）就返回 `None`。
    pub fn new(model: &'a CharLm, vocab: &'a Vocab) -> Option<Self> {
        let sep = vocab.sep()?;
        Some(Self { model, vocab, sep })
    }

    /// 拼音键 → 前 `beam` 条候选，按分数降序。`max_chars` 是生成长度上限。
    pub fn convert(
        &self,
        keys: &str,
        beam: usize,
        max_chars: usize,
    ) -> Result<Vec<Candidate>, NeuralError> {
        let beam = beam.max(1);
        let mut prefix = Vec::with_capacity(keys.len() + 2);
        prefix.push(EOS);
        prefix.extend(self.vocab.encode(keys));
        prefix.push(self.sep);

        let context = self.model.config().context;
        if prefix.len() + max_chars > context {
            return Err(NeuralError::Corrupt(
                "含章·通变的拼音和生成长度超出模型上下文",
            ));
        }

        let device = self.model.device();
        let split = prefix.len() - 1;
        // 前缀最后一位留给 step 走，这样不用为空缓存分支
        let mut cache = self.model.prefix_cache(&prefix[..split])?;
        let last = Tensor::from_vec(prefix[split..].to_vec(), (1, 1), device)?;
        let (mut log_probs, mut grown) = self.model.step(&cache, &last)?;

        let mut beams: Vec<(f64, Vec<u32>)> = vec![(0.0, Vec::new())];
        let mut done: Vec<(f64, Vec<u32>)> = Vec::new();

        for _ in 0..max_chars {
            let rows = log_probs.to_vec2::<f32>()?;
            let mut pool: Vec<(f64, usize, u32)> = Vec::new();
            for (index, (row, (score, _))) in rows.iter().zip(&beams).enumerate() {
                for (token, &lp) in top_k(row, beam) {
                    pool.push((score + f64::from(lp), index, token));
                }
            }
            pool.sort_by(|a, b| b.0.total_cmp(&a.0));

            let mut next: Vec<(f64, Vec<u32>)> = Vec::new();
            let mut order: Vec<u32> = Vec::new();
            let mut tokens: Vec<u32> = Vec::new();
            for (score, index, token) in pool {
                if token == EOS {
                    done.push((score, beams[index].1.clone()));
                    continue;
                }
                if next.len() >= beam {
                    continue;
                }
                let mut text = beams[index].1.clone();
                text.push(token);
                next.push((score, text));
                order.push(index as u32);
                tokens.push(token);
            }
            if next.is_empty() || done.len() >= beam {
                beams = next;
                break;
            }

            let width = next.len();
            let picked = Tensor::from_vec(order, width, device)?;
            cache = grown.select(&picked)?;
            let idx = Tensor::from_vec(tokens, (width, 1), device)?;
            let stepped = self.model.step(&cache, &idx)?;
            log_probs = stepped.0;
            grown = stepped.1;
            beams = next;
        }

        // 没走到 <eos> 的也收进来，短拼音常常 beam 还没耗完就够了
        done.extend(beams);
        done.sort_by(|a, b| b.0.total_cmp(&a.0));

        let mut seen = Vec::new();
        let mut out = Vec::new();
        for (score, ids) in done {
            let text = self.vocab.decode(&ids);
            if text.is_empty() || seen.contains(&text) {
                continue;
            }
            seen.push(text.clone());
            out.push(Candidate { text, score });
            if out.len() >= beam {
                break;
            }
        }
        Ok(out)
    }
}

/// 一行 log 概率里最大的 `k` 个，返回 `(token, log 概率)`。
fn top_k(row: &[f32], k: usize) -> Vec<(u32, &f32)> {
    let mut best: Vec<(u32, &f32)> = Vec::with_capacity(k + 1);
    for (token, lp) in row.iter().enumerate() {
        if best.len() < k {
            best.push((token as u32, lp));
            if best.len() == k {
                best.sort_by(|a, b| b.1.total_cmp(a.1));
            }
        } else if lp > best[k - 1].1 {
            best[k - 1] = (token as u32, lp);
            best.sort_by(|a, b| b.1.total_cmp(a.1));
        }
    }
    if best.len() < k {
        best.sort_by(|a, b| b.1.total_cmp(a.1));
    }
    best
}
