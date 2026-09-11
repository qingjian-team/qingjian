use std::path::Path;

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;

use crate::vocab::EOS;
use crate::{CharLm, ModelConfig, NeuralError, Vocab};

/// 加载好的模型 + 字表：给「前文 + 候选」打分。
pub struct CharScorer {
    model: CharLm,
    vocab: Vocab,
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
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[weights], DType::F32, &device)? };
        let model = CharLm::load(vb, cfg, device)?;
        tracing::info!(
            layers = model.config().n_layer,
            hidden = model.config().n_embd,
            vocab = vocab.len(),
            "神经语言模型已加载"
        );
        Ok(Self { model, vocab })
    }

    pub fn vocab(&self) -> &Vocab {
        &self.vocab
    }

    /// 每个候选接在 `context` 后面的 `log P(候选 | 前文)`，按字累加。
    /// 序列是 `<eos> + 前文 + 候选`，超过模型上下文时从左边截（与训练脚本 `score.py` 一致）；
    /// 几个候选拼成一个 batch、末尾补 0 对齐，一次前向。
    pub fn score(&self, context: &str, texts: &[&str]) -> Result<Vec<f64>, NeuralError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let limit = self.model.config().context;
        let prefix: Vec<u32> = std::iter::once(EOS)
            .chain(self.vocab.encode(context))
            .collect();
        // (序列, 候选占末尾几个 token)
        let sequences: Vec<(Vec<u32>, usize)> = texts
            .iter()
            .map(|text| {
                let tail = self.vocab.encode(text);
                let n = tail.len();
                let mut ids = prefix.clone();
                ids.extend(tail);
                if ids.len() > limit {
                    ids.drain(..ids.len() - limit);
                }
                (ids, n.min(limit.saturating_sub(1)))
            })
            .collect();
        let width = sequences
            .iter()
            .map(|(ids, _)| ids.len())
            .max()
            .unwrap_or(0);
        let batch = sequences.len();
        let mut flat = vec![0u32; batch * width];
        for (row, (ids, _)) in sequences.iter().enumerate() {
            flat[row * width..row * width + ids.len()].copy_from_slice(ids);
        }
        let idx = Tensor::from_vec(flat, (batch, width), self.model.device())?;
        let lp = self.model.log_probs(&idx)?;
        // 只取要的那几个位置：目标 token 在位置 p，用位置 p-1 的分布
        let mut targets = vec![0u32; batch * width];
        for (row, (ids, _)) in sequences.iter().enumerate() {
            for (p, &id) in ids.iter().enumerate().skip(1) {
                targets[row * width + p - 1] = id;
            }
        }
        let targets = Tensor::from_vec(targets, (batch, width, 1), self.model.device())?;
        let picked = lp.gather(&targets, 2)?.squeeze(2)?.to_vec2::<f32>()?;
        Ok(sequences
            .iter()
            .zip(picked)
            .map(|((ids, n), row)| {
                let len = ids.len();
                row[len - 1 - n..len - 1]
                    .iter()
                    .map(|&v| f64::from(v))
                    .sum()
            })
            .collect())
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
            println!("{label}: {:.2} ms", start.elapsed().as_secs_f64() * 100.0);
        }
    }
}
