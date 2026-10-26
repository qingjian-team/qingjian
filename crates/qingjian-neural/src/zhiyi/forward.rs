use super::input::Sequence;
use crate::NeuralError;

/// 模型的前向：一条输入序列 → 每个候选标记一个分。分词、对齐与序列拼法两个实现共用，只有这一步不同。
pub(super) trait Forward: Send + Sync {
    fn logits(&self, sequence: &Sequence) -> Result<Vec<f32>, NeuralError>;
}
