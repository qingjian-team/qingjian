//! 两个语言模型按概率插值（整句词图里是知意三元模型配我们的二元模型）：P = λ·P(甲) + (1 − λ)·P(乙)；一边不认识这个词时只用另一边，两边都不认识交回 Core 兜底。

use qingjian_core::sentence::LanguageModel;

/// 整句词图里三元模型（`zh.ngram`）对二元模型（`lm.qj`）的权重 λ，取值见 `docs/notes/crate-notes.md`（qingjian-lm 一节）。
pub const TRIGRAM_WEIGHT: f64 = 0.5;

pub struct MixedModel {
    first: Box<dyn LanguageModel>,
    second: Box<dyn LanguageModel>,

    /// `first` 的权重 λ（0 到 1）。
    weight: f64,
}

impl MixedModel {
    pub fn new(first: Box<dyn LanguageModel>, second: Box<dyn LanguageModel>, weight: f64) -> Self {
        Self {
            first,
            second,
            weight: weight.clamp(0.0, 1.0),
        }
    }

    fn mix(&self, first: Option<f64>, second: Option<f64>) -> Option<f64> {
        match (first, second) {
            (Some(a), Some(b)) => {
                let p = self.weight * a.exp() + (1.0 - self.weight) * b.exp();
                Some(p.max(f64::MIN_POSITIVE).ln())
            }
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        }
    }
}

impl LanguageModel for MixedModel {
    /// 先列 `first` 的，再补 `second` 里没有的。
    fn successors(&self, previous: &str, limit: usize) -> Vec<String> {
        let mut words = self.first.successors(previous, limit);
        for word in self.second.successors(previous, limit) {
            if words.len() >= limit {
                break;
            }
            if !words.contains(&word) {
                words.push(word);
            }
        }
        words
    }

    fn log_prob(&self, previous: Option<&str>, word: &str) -> Option<f64> {
        self.mix(
            self.first.log_prob(previous, word),
            self.second.log_prob(previous, word),
        )
    }

    fn log_prob_trigram(
        &self,
        earlier: Option<&str>,
        previous: Option<&str>,
        word: &str,
    ) -> Option<f64> {
        self.mix(
            self.first.log_prob_trigram(earlier, previous, word),
            self.second.log_prob_trigram(earlier, previous, word),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 只认一个词、给固定概率的模型。
    struct Fixed(&'static str, f64);

    impl LanguageModel for Fixed {
        fn log_prob(&self, _previous: Option<&str>, word: &str) -> Option<f64> {
            (word == self.0).then(|| self.1.ln())
        }

        fn successors(&self, _previous: &str, _limit: usize) -> Vec<String> {
            vec![self.0.to_owned(), "都有".to_owned()]
        }
    }

    #[test]
    fn successors_list_the_first_model_then_fill_from_the_second() {
        let model = MixedModel::new(Box::new(Fixed("甲", 0.4)), Box::new(Fixed("乙", 0.1)), 0.5);
        assert_eq!(model.successors("前", 8), ["甲", "都有", "乙"]);
        assert_eq!(model.successors("前", 2), ["甲", "都有"]);
    }

    #[test]
    fn mixes_probabilities_and_falls_back_to_the_side_that_knows_the_word() {
        let model = MixedModel::new(Box::new(Fixed("词", 0.4)), Box::new(Fixed("词", 0.1)), 0.5);
        let mixed = model.log_prob(None, "词").unwrap();
        assert!((mixed - 0.25f64.ln()).abs() < 1e-9, "{mixed}");

        let one_sided =
            MixedModel::new(Box::new(Fixed("甲", 0.4)), Box::new(Fixed("乙", 0.1)), 0.5);
        assert!((one_sided.log_prob(None, "甲").unwrap() - 0.4f64.ln()).abs() < 1e-9);
        assert!((one_sided.log_prob(None, "乙").unwrap() - 0.1f64.ln()).abs() < 1e-9);
        assert_eq!(one_sided.log_prob(None, "丙"), None);
    }
}
