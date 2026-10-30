//! 一次等待模型重生成候选译文的状态。

use qingjian_core::Candidate;

pub struct CandidateTranslation {
    pub sequence: u64,

    pub candidate: Candidate,

    /// 独立输入法候选要求的译词语言；有值时只更新当前会话，不写中文 Engine 的个人释义表。
    pub independent_translation: Option<qingjian_core::Language>,
}
