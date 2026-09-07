use serde::{Deserialize, Serialize};

use super::Candidate;

/// 排好序的候选列表，平台层按顺序绘制。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateList {
    /// 候选，索引 0 为首选。
    pub items: Vec<Candidate>,
}
