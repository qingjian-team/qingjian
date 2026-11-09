//! 把三元模型当整句词图的语言模型（[`LanguageModel`]）：逐词问 ln P，词表里没有的词交回 Core 兜底。

use qingjian_core::sentence::LanguageModel;

use super::TrigramModel;

impl LanguageModel for TrigramModel {
    fn log_prob(&self, previous: Option<&str>, word: &str) -> Option<f64> {
        self.log_prob_trigram(None, previous, word)
    }

    fn log_prob_trigram(
        &self,
        earlier: Option<&str>,
        previous: Option<&str>,
        word: &str,
    ) -> Option<f64> {
        let word = self.word(word)?;
        // 前文里的词不在词表时当作没有那么长的前文
        let previous = previous.and_then(|p| self.word(p));
        let earlier = previous.and(earlier.and_then(|e| self.word(e)));
        Some(self.log10(earlier, previous, word) * std::f64::consts::LN_10)
    }
}
