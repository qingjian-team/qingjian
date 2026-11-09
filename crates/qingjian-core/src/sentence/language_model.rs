/// 整句转换的打分来源：词级语言模型。实现放兄弟 crate（`qingjian-lm`），Core 只认这个 trait。
pub trait LanguageModel: Send {
    /// `log P(word | previous)`；`previous` 为 `None` 表示句首。模型不认识 `word` 时返回 `None`，
    /// 由 Core 用词库词频兜底。
    fn log_prob(&self, previous: Option<&str>, word: &str) -> Option<f64>;

    /// `log P(word | earlier previous)`：三元模型覆盖它，缺省退回 [`Self::log_prob`]。
    fn log_prob_trigram(
        &self,
        _earlier: Option<&str>,
        previous: Option<&str>,
        word: &str,
    ) -> Option<f64> {
        self.log_prob(previous, word)
    }

    /// 最常跟在 `previous` 后面的词，最多 `limit` 个、最常见的在前（上屏后的联想用）；模型列不出来时为空。
    fn successors(&self, _previous: &str, _limit: usize) -> Vec<String> {
        Vec::new()
    }

    /// 没接模型（什么词都不认识），「模型认不认识这个词」就没有意义。
    fn is_empty(&self) -> bool {
        false
    }
}

/// 没接语言模型：一律兜底，整句转换退化为一元词频。
#[derive(Debug, Default, Clone, Copy)]
pub struct NoLanguageModel;

impl LanguageModel for NoLanguageModel {
    fn log_prob(&self, _previous: Option<&str>, _word: &str) -> Option<f64> {
        None
    }

    fn is_empty(&self) -> bool {
        true
    }
}
