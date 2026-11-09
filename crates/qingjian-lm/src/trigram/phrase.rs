//! 把三元模型接到 Core 的 [`PhraseModel`] 上。

use qingjian_core::sentence::PhraseModel;

use super::TrigramModel;

impl PhraseModel for TrigramModel {
    fn score(&self, context: &str, texts: &[&str]) -> Vec<f64> {
        self.score_texts(context, texts)
    }
}
