use super::input::Sequence;
use crate::NeuralError;

/// 模型的前向：一条输入序列 → 每个候选标记一个分。分词、对齐与序列拼法两个实现共用，只有这一步不同。
pub(super) trait Forward: Send + Sync {
    fn logits(&self, sequence: &Sequence) -> Result<Vec<f32>, NeuralError>;

    /// composition 头：每个音节槽位一行字表大小的 logits；没有这个头的实现返回 `None`。
    fn char_logits(&self, _sequence: &Sequence) -> Result<Option<Vec<Vec<f32>>>, NeuralError> {
        Ok(None)
    }
}
