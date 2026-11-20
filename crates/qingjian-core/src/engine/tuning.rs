//! Engine 的重打分与整句搜索参数（只有回放调参会改）。

use crate::correction::TypoCosts;
use crate::sentence::{Interpolation, SearchLimits};

/// 重打分与整句转换的可调参数。
pub(crate) struct EngineTuning {
    /// 壳给的应用里光标前的文本；`None` 时前文用本会话历史。
    pub(crate) rescoring_before: Option<String>,

    /// 重打分时神经得分的权重 λ：最终分 = 路径分 + λ·(神经分 − 静态分)。
    pub(crate) neural_weight: f64,

    /// 只有路径分与最优路径差距在这么多 nat 以内的路径才参与重排：差距大的多半是个人 n-gram 拉开的，通用模型不该翻盘。
    pub(crate) neural_margin: f64,

    /// 重打分给模型看的前文长度（本会话最近上屏的字符数），0 为不给前文。
    pub(crate) neural_context: usize,

    /// 个人 n-gram 与静态模型插值的参数；只有回放调参会改（`set_interpolation`），壳用缺省值。
    pub(crate) interpolation: Interpolation,

    /// 整句转换的搜索规模；同上，只有回放调参会改（`set_search_limits`）。
    pub(crate) search_limits: SearchLimits,

    /// 敲错纠正的代价；同上，只有回放调参会改（`set_typo_costs`）。
    pub(crate) typo_costs: TypoCosts,
}

impl Default for EngineTuning {
    fn default() -> Self {
        Self {
            rescoring_before: None,
            neural_weight: super::NEURAL_WEIGHT,
            neural_margin: super::NEURAL_MARGIN,
            neural_context: super::RESCORE_CONTEXT_CHARS,
            interpolation: Interpolation::DEFAULT,
            search_limits: SearchLimits::DEFAULT,
            typo_costs: TypoCosts::DEFAULT,
        }
    }
}
