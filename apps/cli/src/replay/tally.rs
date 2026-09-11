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

    /// 当时就命中首选的条数（日志里 `index == 0`），与「现在」无关，是隐式信号与重排对比的分母。
    pub then_top1: usize,

    /// 当时命中首选的那些：翻页数之和、耗时之和（毫秒）。
    pub hit_pages: usize,
    pub hit_ms: u64,

    /// 当时没命中首选的那些：翻页数之和、耗时之和。
    pub miss_pages: usize,
    pub miss_ms: u64,

    /// 当时经过神经重排的条数，与其中当时命中首选的条数。
    pub rescored_then: usize,
    pub rescored_then_top1: usize,
}

impl Tally {
    /// 从日志里的一条记下当时的情况（命中、翻页、耗时、重排），回放前调。
    pub fn note_logged(&mut self, commit: &qingjian_core::CommitEntry) {
        let hit = commit.index == Some(0);
        if hit {
            self.then_top1 += 1;
            self.hit_pages += commit.pages as usize;
            self.hit_ms += commit.ms;
        } else {
            self.miss_pages += commit.pages as usize;
            self.miss_ms += commit.ms;
        }
        if commit.rescored {
            self.rescored_then += 1;
            self.rescored_then_top1 += usize::from(hit);
        }
    }

    /// 当时命中 / 没命中两组的平均翻页数与平均耗时；日志里没这两个字段（旧格式）时全是 0，返回 `None`。
    pub fn implicit_signal(&self) -> Option<((f64, f64), (f64, f64))> {
        let misses = self.total.saturating_sub(self.then_top1);
        if self.hit_ms + self.miss_ms == 0 || self.then_top1 == 0 || misses == 0 {
            return None;
        }
        let hit = (
            self.hit_pages as f64 / self.then_top1 as f64,
            self.hit_ms as f64 / self.then_top1 as f64,
        );
        let miss = (
            self.miss_pages as f64 / misses as f64,
            self.miss_ms as f64 / misses as f64,
        );
        Some((hit, miss))
    }
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
