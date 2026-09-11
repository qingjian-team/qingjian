/// 整句路径的第二打分来源：给「前文 + 整句文本」按字打 log 概率的模型（字级 Transformer，实现在 `qingjian-neural`）。
/// Core 只认这个 trait；Viterbi 出的前几条路径用它重打分，与路径本身的得分对数线性插值。
pub trait SentenceScorer: Send {
    /// 每条 `texts` 接在 `context`（光标前文，可空）后面的 `log P(text | context)`，与 `texts` 一一对应。
    /// 算不了（模型出错）返回空 Vec，调用方就当没有这个打分。
    fn score(&self, context: &str, texts: &[&str]) -> Vec<f64>;
}
