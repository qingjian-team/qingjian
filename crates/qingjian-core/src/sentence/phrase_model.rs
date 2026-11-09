//! 短语级语言模型：给「接在上文后面的一段文字」打 ln P，选词重排在盖住整段输入的候选之间比较时用。
//! 与整句词图逐词问的 [`LanguageModel`](super::LanguageModel) 不同，它自己切词、自己管上文多长。

/// 短语级语言模型（实现在兄弟 crate，例如 `qingjian-lm` 的三元模型）。
pub trait PhraseModel: Send + Sync {
    /// 每段 `texts` 接在 `context` 后面的 ln P(text | context)，与 `texts` 一一对应。
    fn score(&self, context: &str, texts: &[&str]) -> Vec<f64>;
}
