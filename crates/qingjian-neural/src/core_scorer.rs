use qingjian_core::sentence::SentenceScorer;

use crate::CharScorer;

impl SentenceScorer for CharScorer {
    fn score(&self, context: &str, texts: &[&str]) -> Vec<f64> {
        match CharScorer::score(self, context, texts) {
            Ok(scores) => scores,
            Err(error) => {
                tracing::warn!(%error, "神经重打分失败，本次不用");
                Vec::new()
            }
        }
    }
}
