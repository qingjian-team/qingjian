/// 一类来源的计数。
#[derive(Debug, Default, Clone, Copy)]
pub struct Tally {
    /// 日志里这类上屏的总数。
    pub total: usize,

    /// 现在的首选就是当时选的。
    pub top1: usize,

    /// 在第 2 到第 5 位。
    pub top5: usize,

    /// 在第 6 位之后。
    pub found_later: usize,

    /// 当时选的词现在根本不在候选里（词库变了、删过词、云端词）。
    pub missing: usize,

    /// 作用域现在切不动（双拼方案 / 模式键变了）。
    pub unparsable: usize,

    /// 找到时的名次之和（算平均名次）。
    pub rank_sum: usize,

    /// 当时拼写纠错生效的条数。
    pub corrected_then: usize,

    /// 其中现在仍然纠错的条数。
    pub corrected_now: usize,
}

impl Tally {
    /// 现在能评的条数（找到 + 没找到，不含切不动的）。
    pub fn evaluated(&self) -> usize {
        self.total - self.unparsable
    }

    pub fn found(&self) -> usize {
        self.top1 + self.top5 + self.found_later
    }

    pub fn mean_rank(&self) -> Option<f64> {
        (self.found() > 0).then(|| self.rank_sum as f64 / self.found() as f64)
    }
}
