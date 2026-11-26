//! safetensors 版，candle 跑（macOS 走 Metal）。

use std::path::Path;

use candle_core::Tensor;
use serde::Deserialize;

use super::encoder::Encoder;
use super::forward::Forward;
use super::head::ChoiceHead;
use super::input::{SEP, Sequence};
use super::mask::Masks;
use super::read_json;
use super::weights::Weights;
use crate::NeuralError;

/// `config.json` 里用得上的部分。
#[derive(Deserialize)]
struct Config {
    encoder: EncoderConfig,

    head: HeadConfig,

    /// 开头一段（到第一个 `<eos>`）只看自己，见 [`Masks`]；L12 没有。
    #[serde(default)]
    prefix_mask: bool,

    /// 选词头与整句共用（feather 起）。
    #[serde(default)]
    shared_head: bool,
}

#[derive(Deserialize)]
struct EncoderConfig {
    layer_types: Vec<String>,

    hidden_size: usize,

    intermediate_size: usize,
}

#[derive(Deserialize)]
struct HeadConfig {
    block_count: usize,
}

pub(super) struct CandleForward {
    encoder: Encoder,
    head: ChoiceHead,
    prefix_mask: bool,
}

impl CandleForward {
    /// 加载时把线性层量化成 Q8_0。
    pub fn load(dir: &Path) -> Result<Self, NeuralError> {
        let device = crate::device::default_device()?;
        let config: Config = read_json(&dir.join("config.json"))?;
        let sliding: Vec<bool> = config
            .encoder
            .layer_types
            .iter()
            .map(|kind| kind == "sliding_attention")
            .collect();
        let width = config.encoder.hidden_size;
        let weights = Weights::load(&dir.join("model.safetensors"), &device)?;
        Ok(Self {
            encoder: Encoder::load(
                &weights,
                &sliding,
                (width, config.encoder.intermediate_size),
                &device,
            )?,
            head: ChoiceHead::load(&weights, config.head.block_count, width, config.shared_head)?,
            prefix_mask: config.prefix_mask,
        })
    }

    /// 编码器加头，`[T, 宽度]`。
    fn hidden(&self, sequence: &Sequence) -> Result<Tensor, NeuralError> {
        let t = sequence.ids.len();
        let prefix = self.prefix_mask.then(|| {
            sequence
                .ids
                .iter()
                .position(|&id| id == SEP)
                .map_or(t, |i| i + 1)
        });
        let masks = Masks::new(t, prefix, self.encoder.device())?;
        let encoded =
            self.encoder
                .forward(&sequence.ids, &sequence.units, &sequence.markers, &masks)?;
        Ok(self.head.hidden(&encoded, masks.global.as_ref())?)
    }
}

impl Forward for CandleForward {
    fn logits(&self, sequence: &Sequence) -> Result<Vec<f32>, NeuralError> {
        let h = self.hidden(sequence)?;
        Ok(self.head.scores(&h, &sequence.markers)?)
    }

    fn char_logits(&self, sequence: &Sequence) -> Result<Option<Vec<Vec<f32>>>, NeuralError> {
        let slots: Vec<u32> = (0..sequence.units.len() as u32)
            .filter(|&i| sequence.units[i as usize] != 0)
            .collect();
        if slots.is_empty() {
            return Ok(None);
        }
        let h = self.hidden(sequence)?;
        Ok(self.head.char_logits(&h, &slots)?)
    }
}
