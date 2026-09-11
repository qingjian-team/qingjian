use std::path::Path;
use std::sync::Mutex;

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;

use crate::model::PrefixCache;
use crate::vocab::EOS;
use crate::{CharLm, ModelConfig, NeuralError, Vocab};

/// 加载好的模型 + 字表：给「前文 + 候选」打分。
pub struct CharScorer {
    model: CharLm,
    vocab: Vocab,

    /// 最近一段前文的 K / V 缓存（前文 token 与缓存）：一次组句里前文不变，候选换了只算候选。
    cache: Mutex<Option<(Vec<u32>, PrefixCache)>>,
}

impl CharScorer {
    /// 从导出目录加载（`model.safetensors` / `config.json` / `vocab.json`）。
    pub fn load(dir: &Path) -> Result<Self, NeuralError> {
        let device = default_device()?;
        let config_path = dir.join("config.json");
        let text = std::fs::read_to_string(&config_path).map_err(|source| NeuralError::Io {
            path: config_path.clone(),
            source,
        })?;
        let cfg: ModelConfig = serde_json::from_str(&text).map_err(|source| NeuralError::Json {
            path: config_path,
            source,
        })?;
        let vocab = Vocab::load(&dir.join("vocab.json"))?;
        if vocab.len() != cfg.vocab_size {
            return Err(NeuralError::Corrupt("vocab.json size differs from config"));
        }
        let weights = dir.join("model.safetensors");
        // SAFETY：mmap 的权重文件在模型存活期间不改动
        let dtype = weight_dtype();
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[weights], dtype, &device)? };
        let model = CharLm::load(vb, cfg, device)?;
        tracing::info!(
            layers = model.config().n_layer,
            hidden = model.config().n_embd,
            vocab = vocab.len(),
            "神经语言模型已加载"
        );
        Ok(Self {
            model,
            vocab,
            cache: Mutex::new(None),
        })
    }

    pub fn vocab(&self) -> &Vocab {
        &self.vocab
    }

    /// 每个候选接在 `context` 后面的 `log P(候选 | 前文)`，按字累加。
    /// 序列是 `<eos> + 前文 + 候选`；前文（不含最后一个 token）的 K / V 走缓存，同一段前文只算一次，
    /// 每个候选只算「前文最后一个 token + 候选」这一小段；几个候选拼成一个 batch、末尾补 0 对齐，一次前向。
    /// 前文加最长候选超过模型上下文时前文从左边截（与训练脚本 `score.py` 一致）。
    pub fn score(&self, context: &str, texts: &[&str]) -> Result<Vec<f64>, NeuralError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let limit = self.model.config().context;
        let full: Vec<u32> = std::iter::once(EOS)
            .chain(self.vocab.encode(context))
            .collect();
        // 候选最长不能超过上下文减一（还要留前文的最后一个 token）：再长的从开头截
        let tails: Vec<Vec<u32>> = texts
            .iter()
            .map(|text| {
                let mut ids = self.vocab.encode(text);
                if ids.len() > limit - 1 {
                    ids.drain(..ids.len() - (limit - 1));
                }
                ids
            })
            .collect();
        let longest = tails.iter().map(Vec::len).max().unwrap_or(0);
        let width = longest + 1;
        // 缓存的是前文去掉最后一个 token 的部分，最后一个 token 放进每一行的开头，它的输出分布给候选第一个字用
        let (last, head) = full.split_last().expect("has eos");
        let head = &head[head.len().saturating_sub(limit - width)..];
        let batch = tails.len();
        let mut flat = vec![0u32; batch * width];
        let mut targets = vec![0u32; batch * width];
        for (row, tail) in tails.iter().enumerate() {
            flat[row * width] = *last;
            flat[row * width + 1..row * width + 1 + tail.len()].copy_from_slice(tail);
            targets[row * width..row * width + tail.len()].copy_from_slice(tail);
        }
        let idx = Tensor::from_vec(flat, (batch, width), self.model.device())?;
        let targets = Tensor::from_vec(targets, (batch, width, 1), self.model.device())?;
        let mut guard = self
            .cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if guard.as_ref().is_none_or(|(ids, _)| ids != head) {
            *guard = Some((head.to_vec(), self.model.prefix_cache(head)?));
        }
        let (_, cache) = guard.as_ref().expect("filled above");
        let lp = self.model.log_probs_after(cache, &idx)?;
        drop(guard);
        let picked = lp.gather(&targets, 2)?.squeeze(2)?.to_vec2::<f32>()?;
        Ok(tails
            .iter()
            .zip(picked)
            .map(|(tail, row)| row[..tail.len()].iter().map(|&v| f64::from(v)).sum())
            .collect())
    }
}

/// 权重与中间量的精度：Metal 上缺省 f16（与 f32 打分一致，显存减一半、略快），CPU 上 f32（candle 的 CPU f16 矩阵乘慢）；
/// 环境变量 `QINGJIAN_NEURAL_DTYPE=f32|f16` 可强制。
fn weight_dtype() -> DType {
    match std::env::var("QINGJIAN_NEURAL_DTYPE").as_deref() {
        Ok("f16") => DType::F16,
        Ok("f32") => DType::F32,
        _ if cfg!(feature = "metal") => DType::F16,
        _ => DType::F32,
    }
}

#[cfg(not(feature = "metal"))]
fn default_device() -> Result<Device, NeuralError> {
    Ok(Device::Cpu)
}

#[cfg(feature = "metal")]
fn default_device() -> Result<Device, NeuralError> {
    Ok(Device::new_metal(0)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn export_dir() -> Option<std::path::PathBuf> {
        let dir =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/lm-train/export/full-small");
        dir.join("model.safetensors").exists().then_some(dir)
    }

    /// 与训练脚本 `score.py` 对拍：同一模型、同一序列，log 概率要一致（数值差在 fp16 权重转 f32 的误差内）。
    #[test]
    fn matches_the_python_scorer() {
        let Some(dir) = export_dir() else {
            eprintln!("没有导出的模型，跳过");
            return;
        };
        let scorer = CharScorer::load(&dir).unwrap();
        let scores = scorer
            .score("我今天想去", &["上海", "伤害", "吃饭"])
            .unwrap();
        assert!((scores[0] - -4.981).abs() < 0.05, "{scores:?}");
        assert!((scores[1] - -13.113).abs() < 0.05, "{scores:?}");
        assert!((scores[2] - -6.491).abs() < 0.05, "{scores:?}");
        // 单独算与批量算一致
        let alone = scorer.score("我今天想去", &["伤害"]).unwrap();
        assert!(
            (alone[0] - scores[1]).abs() < 1e-3,
            "{alone:?} vs {scores:?}"
        );
        // 前文超长时从左截，不报错
        let long: String = "很长的前文。".repeat(40);
        assert!(scorer.score(&long, &["上海"]).unwrap()[0] < 0.0);
        // 换过前文再换回来（缓存重算）结果不变；候选比上下文还长也不报错
        let again = scorer.score("我今天想去", &["上海"]).unwrap();
        assert!((again[0] - scores[0]).abs() < 1e-3, "{again:?}");
        let huge: String = "字".repeat(300);
        assert!(scorer.score("", &[huge.as_str()]).unwrap()[0] < 0.0);
    }
}

#[cfg(test)]
mod latency {
    use super::*;

    /// 延迟探针：前文 64 字 + 8 条约 8 字的候选，一次 batch。`cargo test --release -p qingjian-neural -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn batch_latency() {
        let dir =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/lm-train/export/full-small");
        let scorer = CharScorer::load(&dir).unwrap();
        let context: String =
            "今天下午的会议讨论了输入法的排序问题，大家觉得整句转换还可以再准一些，".repeat(2);
        let context: String = context.chars().take(64).collect();
        let texts = [
            "我们明天再讨论一下",
            "我们明天在讨论一下",
            "我们名天再讨论一下",
            "我门明天再讨论一下",
            "我们明天再讨论以下",
            "我们明天再讨论一夏",
            "我们明天再讨论移下",
            "我们明天再讨论亦下",
        ];
        for (label, ctx, n) in [
            ("空前文 1 条", "", 1),
            ("空前文 8 条", "", 8),
            ("64 字前文 1 条", context.as_str(), 1),
            ("64 字前文 8 条", context.as_str(), 8),
        ] {
            let _ = scorer.score(ctx, &texts[..n]).unwrap();
            let start = std::time::Instant::now();
            for _ in 0..10 {
                let _ = scorer.score(ctx, &texts[..n]).unwrap();
            }
            println!(
                "{label}（前文已缓存）: {:.2} ms",
                start.elapsed().as_secs_f64() * 100.0
            );
            let start = std::time::Instant::now();
            for i in 0..10 {
                // 每次换一段前文，逼它重算缓存
                let fresh = format!("{ctx}{i}");
                let _ = scorer.score(&fresh, &texts[..n]).unwrap();
            }
            println!(
                "{label}（前文重算）: {:.2} ms",
                start.elapsed().as_secs_f64() * 100.0
            );
        }
    }
}
