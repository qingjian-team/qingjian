//! 选词模型的接口。

/// 选词模型：给盖住整段输入的几条候选各打一个分，壳在停顿后经后台线程调用（见 `engine::choice` 的模块说明）。
///
/// 对齐在按键回调里同步做（便宜，只查表），打分在后台线程做（一页几十毫秒）。
pub trait ChoiceModel: Send + Sync {
    /// 中文候选：按它的音节把敲的键对成模型的输入单元；盖不住整段输入（只对上一部分、对不上）返回 `None`，不参与比较。
    fn align(&self, keys: &str, syllables: &[String]) -> Option<Vec<u32>>;

    /// 英文补全：敲的字母的输入单元。
    fn letter_units(&self, keys: &str) -> Vec<u32>;

    /// 每条候选的分（越大越合适），与 `texts` 一一对应；算不了返回空 Vec。
    fn score(&self, context: &str, typed: &[u32], texts: &[&str]) -> Vec<f32>;
}
