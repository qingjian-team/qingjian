//! Marian 编码器与解码器在当前进程内推理；纯 CPU，随时检查取消状态。

use std::path::Path;
use std::sync::OnceLock;

use ort::session::{Session, builder::GraphOptimizationLevel};
use ort::value::Tensor;
use tokenizers::Tokenizer;

use crate::OfflineError;
use crate::generation::Generation;

pub struct Backend {
    encoder: Session,

    decoder: Session,

    tokenizer: Tokenizer,

    generation: Generation,
}

fn error(error: impl std::fmt::Display) -> OfflineError {
    OfflineError::Inference(error.to_string())
}

impl Backend {
    pub fn load(dir: &Path, runtime: &Path) -> Result<Self, OfflineError> {
        static RUNTIME: OnceLock<Result<(), String>> = OnceLock::new();
        RUNTIME
            .get_or_init(|| {
                ort::init_from(runtime)
                    .map(|builder| {
                        builder.with_name("qingjian").with_telemetry(false).commit();
                    })
                    .map_err(|error| error.to_string())
            })
            .clone()
            .map_err(OfflineError::Inference)?;
        let load = |name: &str| -> Result<Session, OfflineError> {
            Session::builder()
                .map_err(error)?
                .with_optimization_level(GraphOptimizationLevel::Level3)
                .map_err(error)?
                .with_intra_threads(2)
                .map_err(error)?
                .commit_from_file(dir.join(name))
                .map_err(error)
        };
        Ok(Self {
            encoder: load("encoder_model_quantized.onnx")?,
            decoder: load("decoder_model_quantized.onnx")?,
            tokenizer: crate::tokenization::load(dir)?,
            generation: serde_json::from_slice(&std::fs::read(dir.join("config.json"))?)?,
        })
    }

    pub fn translate(
        &mut self,
        text: &str,
        target: &str,
        cancelled: impl Fn() -> bool,
    ) -> Result<String, OfflineError> {
        if text.trim().is_empty() || text.encode_utf16().count() > 500 {
            return Err(OfflineError::Invalid(
                "selection must contain 1 to 500 UTF-16 units",
            ));
        }
        if !matches!(target, "en" | "zh") {
            return Err(OfflineError::Invalid("unsupported target"));
        }
        let mut result = String::new();
        for line in text.split_inclusive('\n') {
            let body = line.trim();
            if !body.is_empty() {
                let encoding = self.tokenizer.encode(body, false).map_err(error)?;
                let tokens: Vec<i64> = encoding
                    .get_ids()
                    .iter()
                    .map(|&id| i64::from(id))
                    .filter(|&id| id != self.generation.eos_token_id)
                    .collect();
                let mut parts = Vec::new();
                for chunk in tokens.chunks(180) {
                    parts.push(self.decode(chunk, &cancelled)?);
                }
                result.push_str(&line[..line.len() - line.trim_start().len()]);
                result.push_str(&parts.join(if target == "en" { " " } else { "" }));
                result.push_str(&line[line.trim_end().len()..]);
            } else {
                result.push_str(line);
            }
        }
        if result.trim().is_empty() {
            return Err(OfflineError::Invalid("empty translation"));
        }
        Ok(result)
    }

    fn decode(
        &mut self,
        tokens: &[i64],
        cancelled: &impl Fn() -> bool,
    ) -> Result<String, OfflineError> {
        if cancelled() {
            return Err(OfflineError::Invalid("cancelled"));
        }
        let mut source = tokens.to_vec();
        source.push(self.generation.eos_token_id);
        let row = |ids: Vec<i64>| Tensor::from_array(([1, ids.len()], ids));
        let encoded = self
            .encoder
            .run(ort::inputs![
                "input_ids" => row(source.clone()).map_err(error)?,
                "attention_mask" => row(vec![1; source.len()]).map_err(error)?,
            ])
            .map_err(error)?;
        let (shape, hidden) = encoded[0].try_extract_tensor::<f32>().map_err(error)?;
        let shape = shape.clone();
        let hidden = hidden.to_vec();
        let mut ids = vec![self.generation.decoder_start_token_id];
        for _ in 0..384 {
            if cancelled() {
                return Err(OfflineError::Invalid("cancelled"));
            }
            let outputs = self.decoder.run(ort::inputs![
                "input_ids" => row(ids.clone()).map_err(error)?,
                "encoder_hidden_states" => Tensor::from_array((shape.clone(), hidden.clone())).map_err(error)?,
                "encoder_attention_mask" => row(vec![1; source.len()]).map_err(error)?,
            ]).map_err(error)?;
            let (_, logits) = outputs["logits"]
                .try_extract_tensor::<f32>()
                .map_err(error)?;
            let scores = logits
                .chunks_exact(self.generation.vocab_size)
                .next_back()
                .ok_or(OfflineError::Invalid("invalid decoder output"))?;
            let token = scores
                .iter()
                .enumerate()
                .filter(|(id, score)| {
                    *id as i64 != self.generation.pad_token_id && score.is_finite()
                })
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(id, _)| id as i64)
                .ok_or(OfflineError::Invalid("invalid decoder logits"))?;
            if token == self.generation.eos_token_id {
                let output: Vec<u32> = ids.iter().skip(1).map(|&id| id as u32).collect();
                return self.tokenizer.decode(&output, true).map_err(error);
            }
            ids.push(token);
        }
        Err(OfflineError::Invalid(
            "output limit reached; split the selection",
        ))
    }
}
