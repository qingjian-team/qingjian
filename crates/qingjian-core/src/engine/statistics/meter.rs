use super::{Usage, UsageSummary};

/// 输入统计的累计方。实现放学习 crate，Core 只往里送每次上屏的用量。
pub trait UsageMeter: Send {
    /// 记一次上屏。实现不能拖慢输入：至多往文件末尾追加一小行（进程被杀也不丢），大的整理放 [`Self::flush`]。
    fn record(&mut self, usage: Usage);

    /// 落盘或整理。壳在停用输入法时调用，激活期间也定时调；失败只记日志。
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
