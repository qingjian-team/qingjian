use std::path::Path;
use std::sync::Mutex;

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use qingjian_format::{Container, Kind, Metadata};

use crate::model::PrefixCache;
use crate::vocab::EOS;
use crate::{CharLm, ModelConfig, NeuralError, Vocab, find_model, qjm};

/// 加载好的模型 + 字表：给「前文 + 候选」打分。
pub struct CharScorer {
    model: CharLm,
    vocab: Vocab,

    /// `.qjm` 的 `META`（名称 / 许可证 / 署名）；三件套目录没有。
    metadata: Option<Metadata>,

    /// 最近一段前文的 K / V 缓存（前文 token 与缓存）：一次组句里前文不变，候选换了只算候选。
    cache: Mutex<Option<(Vec<u32>, PrefixCache)>>,
}

impl CharScorer {
    /// 加载模型。`path` 是 `.qjm` 单文件、三件套目录（`model.safetensors` / `config.json` / `vocab.json`），
    /// 或装着其中之一的目录（按 [`find_model`] 挑）。
    pub fn load(path: &Path) -> Result<Self, NeuralError> {
        let source = if path.is_dir() {
            find_model(path).ok_or_else(|| NeuralError::NotFound(path.to_owned()))?
        } else {
            path.to_owned()
        };
        let device = default_device()?;
        let dtype = weight_dtype();
        let scorer = if source.is_dir() {
            Self::load_directory(&source, dtype, device)?
        } else {
            Self::load_packed(&source, dtype, device)?
        };
        tracing::info!(
            source = %source.display(),
            name = scorer.metadata.as_ref().map(|m| m.name.as_str()).unwrap_or_default(),
            layers = scorer.model.config().n_layer,
            hidden = scorer.model.config().n_embd,
            vocab = scorer.vocab.len(),
            "神经语言模型已加载"
        );
        Ok(scorer)
    }

    /// 三件套目录：权重 mmap 给 candle。
    fn load_directory(dir: &Path, dtype: DType, device: Device) -> Result<Self, NeuralError> {
        let config_path = dir.join(qjm::CONFIG_FILE);
        let text = std::fs::read_to_string(&config_path).map_err(|source| NeuralError::Io {
            path: config_path.clone(),
            source,
        })?;
        let cfg = ModelConfig::from_json(&text, &config_path)?;
        let vocab = Vocab::load(&dir.join(qjm::VOCAB_FILE))?;
        // SAFETY：mmap 的权重文件在模型存活期间不改动
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[dir.join(qjm::WEIGHTS_FILE)], dtype, &device)?
        };
        Self::assemble(cfg, vocab, vb, device, None)
    }

    /// `.qjm`：容器 mmap 一次，三节各自切片；张量搬上设备后容器就可以丢了。
    fn load_packed(path: &Path, dtype: DType, device: Device) -> Result<Self, NeuralError> {
        let container = Container::open(path, Kind::Model)?;
        let cfg = ModelConfig::from_json(&container.text(qjm::CONFIG_TAG)?, path)?;
        let vocab = Vocab::from_json(&container.text(qjm::VOCAB_TAG)?, path)?;
        let vb =
            VarBuilder::from_slice_safetensors(container.bytes(qjm::WEIGHTS_TAG)?, dtype, &device)?;
        Self::assemble(cfg, vocab, vb, device, Some(container.metadata().clone()))
    }

    fn assemble(
        cfg: ModelConfig,
        vocab: Vocab,
        vb: VarBuilder,
        device: Device,
        metadata: Option<Metadata>,
    ) -> Result<Self, NeuralError> {
        if vocab.len() != cfg.vocab_size {
            return Err(NeuralError::Corrupt("vocab size differs from config"));
        }
        let model = CharLm::load(vb, cfg, device)?;
        Ok(Self {
            model,
            vocab,
            metadata,
            cache: Mutex::new(None),
        })
    }

    /// `.qjm` 带的元数据（名称 / 许可证 / 署名 / 参数量）；三件套目录加载的没有。
    pub fn metadata(&self) -> Option<&Metadata> {
        self.metadata.as_ref()
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

    /// 随包模型的三件套（训练仓库导出到 `data/model/`）；没有就跳过这些测试。
    fn export_dir() -> Option<std::path::PathBuf> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/model");
        dir.join(qjm::WEIGHTS_FILE).exists().then_some(dir)
    }

    /// 与训练脚本 `score.py` 对拍：同一模型、同一序列，log 概率要一致（数值差在 fp16 权重转 f32 的误差内）。
    #[test]
    fn matches_the_python_scorer() {
        let Some(dir) = export_dir() else {
            eprintln!("没有导出的模型，跳过");
            return;
        };
        let scorer =
            CharScorer::load_directory(&dir, weight_dtype(), default_device().unwrap()).unwrap();
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

    /// 三件套打成 `.qjm` 再加载，分数与目录加载一致，元数据原样带回；目录里有 `.qjm` 时 `load(目录)` 挑的是它。
    #[test]
    fn packed_file_scores_like_the_directory() {
        let Some(dir) = export_dir() else {
            eprintln!("没有导出的模型，跳过");
            return;
        };
        let out_dir = std::env::temp_dir().join("qingjian-neural-tests/packed");
        std::fs::create_dir_all(&out_dir).unwrap();
        let out = out_dir.join("model.qjm");
        let metadata = Metadata {
            name: "测试模型".to_owned(),
            license: "CC-BY-SA-4.0".to_owned(),
            ..Metadata::default()
        };
        let parameters = qjm::pack(&dir, &out, &metadata).unwrap();
        assert!(parameters > 1_000_000, "{parameters}");

        let packed = CharScorer::load(&out).unwrap();
        let meta = packed.metadata().unwrap();
        assert_eq!(meta.name, "测试模型");
        assert_eq!(meta.entries, parameters);
        // 与 load 同一设备同一精度（workspace 一起测时 metal feature 会被统一打开）
        let by_dir =
            CharScorer::load_directory(&dir, weight_dtype(), default_device().unwrap()).unwrap();
        let texts = ["上海", "伤害", "吃饭"];
        let a = packed.score("我今天想去", &texts).unwrap();
        let b = by_dir.score("我今天想去", &texts).unwrap();
        for (x, y) in a.iter().zip(&b) {
            assert!((x - y).abs() < 1e-4, "{a:?} vs {b:?}");
        }
        assert_eq!(find_model(&out_dir), Some(out.clone()));
        assert!(CharScorer::load(&out_dir).unwrap().metadata().is_some());

        // 别的 .qj 种类不认
        let error = CharScorer::load(&dir.join("../generated/lm.qj"))
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        assert!(
            error.contains("wrong data kind") || error.contains("io error"),
            "{error}"
        );
        let _ = std::fs::remove_dir_all(&out_dir);
    }
}

#[cfg(test)]
mod latency {
    use super::*;

    /// 延迟探针：前文 64 字 + 8 条约 8 字的候选，一次 batch。`cargo test --release -p qingjian-neural -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn batch_latency() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/model");
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
