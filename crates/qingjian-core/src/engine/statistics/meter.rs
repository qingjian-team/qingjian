use super::{Usage, UsageSummary};

/// 输入统计的累计方。实现放学习 crate，Core 只往里送每次上屏的用量。
pub trait UsageMeter: Send {
    /// 记一次上屏。实现不能阻塞输入：只加内存里的计数，落盘放 [`Self::flush`]。
    fn record(&mut self, usage: Usage);

    /// 落盘。壳在停用输入法时调用，激活期间也定时调；失败只记日志。
    fn flush(&mut self) {}

    /// 给偏好设置看的汇总。
    fn summary(&self) -> UsageSummary {
        UsageSummary::default()
    }
}

/// 不统计。
#[derive(Debug, Clone, Copy, Default)]
pub struct NoUsageMeter;

impl UsageMeter for NoUsageMeter {
    fn record(&mut self, _usage: Usage) {}
}
