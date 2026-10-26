//! safetensors 版，candle 跑（macOS 走 Metal）。

use std::path::Path;

use serde::Deserialize;

use super::encoder::Encoder;
use super::forward::Forward;
use super::head::ChoiceHead;
use super::input::Sequence;
use super::read_json;
use super::weights::Weights;
use crate::NeuralError;

/// `config.json` 里用得上的部分。
#[derive(Deserialize)]
struct Config {
    encoder: EncoderConfig,
}

#[derive(Deserialize)]
struct EncoderConfig {
    layer_types: Vec<String>,
}

pub(super) struct CandleForward {
    encoder: Encoder,
    head: ChoiceHead,
}

impl CandleForward {
    /// 加载时把线性层量化成 Q8_0。
    pub fn load(dir: &Path) -> Result<Self, NeuralError> {
        let device = crate::scorer::default_device()?;
        let config: Config = read_json(&dir.join("config.json"))?;
        let sliding: Vec<bool> = config
            .encoder
            .layer_types
            .iter()
            .map(|kind| kind == "sliding_attention")
            .collect();
        let weights = Weights::load(&dir.join("model.safetensors"), &device)?;
        Ok(Self {
            encoder: Encoder::load(&weights, &sliding, &device)?,
            head: ChoiceHead::load(&weights)?,
        })
    }
}

impl Forward for CandleForward {
    fn logits(&self, sequence: &Sequence) -> Result<Vec<f32>, NeuralError> {
        let encoded = self.encoder.forward(&sequence.ids, &sequence.units)?;
        Ok(self.head.scores(&encoded, &sequence.markers)?)
    }
}
