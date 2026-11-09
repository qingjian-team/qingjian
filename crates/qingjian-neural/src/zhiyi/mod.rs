//! 试验：知意输入法的选词模型（知意 1.4V2-Long-L12）给候选打分，CLI 回放对比与 Core 的选词重排共用。
//! 两种权重：safetensors 版由 candle 跑（`candle_forward`，结构按模型包里的 README 实现：mmBERT-small 留 12 层的编码器 + 两层选词头），
//! int8 ONNX 版由 ONNX Runtime 跑（`onnx_forward`，Windows 用）。分词、对齐、序列拼法两边共用。
//! 权重 2026-10-20 起是 Apache-2.0，不随包，放用户目录；目录里要有 `tokenizer.json`、`unified_vocab.json`，
//! 再加 `model.safetensors` + `config.json`，或 `laya.int8g.onnx`。

mod bpe;
mod candle_forward;
mod choice_model;
mod encoder;
mod forward;
mod head;
mod input;
#[cfg(feature = "zhiyi-onnx")]
mod onnx_forward;
mod weights;

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use serde::Deserialize;

use crate::NeuralError;
use bpe::Bpe;
use candle_forward::CandleForward;
use forward::Forward;
use input::Sequence;

/// `unified_vocab.json` 里用得上的部分。
#[derive(Deserialize)]
struct UnifiedVocab {
    units: Vec<String>,
    letters: String,

    /// composition 头的字表（`char_logits` 的列）；旧模型包没有。
    #[serde(default)]
    chars: Vec<String>,
}

/// 加载好的选词模型。
pub struct ChoiceScorer {
    forward: Box<dyn Forward>,
    tokenizer: Bpe,
    /// 拼音单元名（`f:zhong` / `p:g`）→ 单元号。
    units: HashMap<String, u32>,
    /// 英文字母表，单元号是 `units.len()` + 下标。
    letters: Vec<char>,

    /// composition 头的字 → 列号。
    chars: Arc<HashMap<char, u32>>,
}

impl ChoiceScorer {
    /// safetensors 版：加载时把线性层量化成 Q8_0，要一两秒。
    pub fn load(dir: &Path) -> Result<Self, NeuralError> {
        Self::with_forward(dir, Box::new(CandleForward::load(dir)?))
    }

    /// int8 ONNX 版：`runtime` 是 ONNX Runtime 动态库（`onnxruntime.dll`），CPU 跑 `threads` 个线程。
    #[cfg(feature = "zhiyi-onnx")]
    pub fn load_onnx(dir: &Path, runtime: &Path, threads: usize) -> Result<Self, NeuralError> {
        let forward = onnx_forward::OnnxForward::load(dir, runtime, threads)?;
        Self::with_forward(dir, Box::new(forward))
    }

    fn with_forward(dir: &Path, forward: Box<dyn Forward>) -> Result<Self, NeuralError> {
        let vocab: UnifiedVocab = read_json(&dir.join("unified_vocab.json"))?;
        let tokenizer = Bpe::load(&dir.join("tokenizer.json"))?;
        Ok(Self {
            forward,
            tokenizer,
            units: vocab
                .units
                .iter()
                .enumerate()
                .map(|(i, name)| (name.clone(), i as u32))
                .collect(),
            letters: vocab.letters.chars().collect(),
            chars: Arc::new(
                vocab
                    .chars
                    .iter()
                    .enumerate()
                    .filter_map(|(i, text)| Some((text.chars().next()?, i as u32)))
                    .collect(),
            ),
        })
    }

    /// 给 `candidates` 各打一个分（越大越合适）：`context` 是光标前的文字，`typed` 是敲的拼音或字母的单元
    /// （[`Self::align`] / [`Self::letter_units`]）。
    pub fn score(
        &self,
        context: &str,
        typed: &[u32],
        candidates: &[&str],
    ) -> Result<Vec<f32>, NeuralError> {
        let sequence = self.sequence(context, typed, candidates)?;
        self.forward.logits(&sequence)
    }

    /// 直接按 token 序列打分（测试向量用）。
    pub fn score_ids(
        &self,
        ids: &[u32],
        units: &[u32],
        markers: &[u32],
    ) -> Result<Vec<f32>, NeuralError> {
        self.forward.logits(&Sequence {
            ids: ids.to_vec(),
            units: units.to_vec(),
            markers: markers.to_vec(),
        })
    }

    /// composition 头的字表：字 → [`Self::compose`] 每行里的列号。
    pub fn char_index(&self) -> Arc<HashMap<char, u32>> {
        Arc::clone(&self.chars)
    }

    /// 按音节出字：每个音节一行、各字的 log 概率（对整张字表做 log softmax，列号见 [`Self::char_index`]）。
    /// 模型没有 composition 头、有音节对不上输入单元时返回 `Ok(None)`。
    pub fn compose(
        &self,
        context: &str,
        syllables: &[(&str, bool)],
    ) -> Result<Option<Vec<Vec<f32>>>, NeuralError> {
        if self.chars.is_empty() || syllables.is_empty() {
            return Ok(None);
        }
        let mut typed = Vec::with_capacity(syllables.len());
        for (text, complete) in syllables {
            let name = format!("{}:{text}", if *complete { 'f' } else { 'p' });
            let Some(&unit) = self.units.get(&name) else {
                return Ok(None);
            };
            typed.push(unit);
        }
        // 选词头要至少一个候选标记，给一个空候选占位；composition 头只看拼音槽位
        let sequence = self.sequence(context, &typed, &[""])?;
        let Some(rows) = self.forward.char_logits(&sequence)? else {
            return Ok(None);
        };
        if rows.len() != syllables.len() {
            return Ok(None);
        }
        Ok(Some(
            rows.into_iter().map(|row| log_softmax(&row)).collect(),
        ))
    }

    fn encode(&self, text: &str) -> Vec<u32> {
        self.tokenizer.encode(text)
    }
}

fn log_softmax(logits: &[f32]) -> Vec<f32> {
    let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let sum: f32 = logits.iter().map(|&l| (l - max).exp()).sum();
    let log_sum = sum.ln();
    logits.iter().map(|&l| l - max - log_sum).collect()
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, NeuralError> {
    let text = std::fs::read_to_string(path).map_err(|source| NeuralError::Io {
        path: path.to_owned(),
        source,
    })?;
    serde_json::from_str(&text).map_err(|source| NeuralError::Json {
        path: path.to_owned(),
        source,
    })
}

#[cfg(test)]
mod tests;
