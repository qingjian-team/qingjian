//! 点中的是什么：第几个候选，或第几个候选的第几条译词。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitTarget {
    /// 这一页第 `n` 个候选（从 0 起）。
    Candidate(usize),

    /// 这一页第 `candidate` 个候选右侧的第 `sense` 条译词（从 0 起，与「上屏第一 / 第二条译词」的键对应）。
    Translation { candidate: usize, sense: usize },
}
