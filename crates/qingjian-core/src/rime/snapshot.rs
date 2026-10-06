//! 一次从 C 上下文复制出的完整显示状态，不暴露借用指针。
use super::RimeMenu;
use crate::CandidateList;

#[derive(Default)]
pub(crate) struct Snapshot {
    pub input: String,

    pub caret: usize,

    pub preedit: String,

    pub candidates: CandidateList,

    pub menu: RimeMenu,
}
