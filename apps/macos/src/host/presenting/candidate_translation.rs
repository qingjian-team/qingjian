//! 一次等待模型重生成候选译文的状态。

use qingjian_core::Candidate;

pub struct CandidateTranslation {
    pub sequence: u64,

    pub candidate: Candidate,
}
