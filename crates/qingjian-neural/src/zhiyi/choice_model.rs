//! 把知意的选词模型接到 Core 的 [`ChoiceModel`] 上。

use qingjian_core::ChoiceModel;

use super::ChoiceScorer;

impl ChoiceModel for ChoiceScorer {
    fn align(&self, keys: &str, syllables: &[String]) -> Option<Vec<u32>> {
        ChoiceScorer::align(self, keys, syllables)
    }

    fn letter_units(&self, keys: &str) -> Vec<u32> {
        ChoiceScorer::letter_units(self, keys)
    }

    fn score(&self, context: &str, typed: &[u32], texts: &[&str]) -> Vec<f32> {
        match ChoiceScorer::score(self, context, typed, texts) {
            Ok(scores) => scores,
            Err(error) => {
                tracing::warn!(%error, "知意选词打分失败，本次不重排");
                Vec::new()
            }
        }
    }
}
