//! Engine 的会话流：上屏历史、连续上屏链与联想请求。

use std::collections::HashMap;

use super::commit::CommitChain;
use super::prediction::PendingPrediction;
use crate::history::InputHistory;

/// 跨按键的会话流状态：上屏历史、提交链、联想请求序号与在飞请求。
#[derive(Default)]
pub(crate) struct EngineFlow {
    /// 本次会话经我们上屏的文本，按应用分开记；应用不给光标前文时给模型当前文。
    pub(crate) history: InputHistory,

    /// 连续上屏的链，个人 n-gram 与自动造词靠它。
    pub(crate) chain: CommitChain,

    /// 最近一次联想请求的序号，0 表示还没发过。
    pub(crate) prediction_sequence: u64,

    /// 尚未返回的请求。组句联想与整句候选翻译可同时在飞，不能只记最后一条。
    pub(crate) pending_predictions: HashMap<u64, PendingPrediction>,
}
