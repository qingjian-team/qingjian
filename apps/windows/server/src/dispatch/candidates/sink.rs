//! 候选窗口输出端的 trait 与空实现。

use qingjian_platform::protocol::{Frame, ScreenRect};

/// Router 在工人线程上调，窗口在 UI 线程上，故要 `Send`。
pub trait CandidateSink: Send {
    /// 把候选窗口摆到 `rect`（组句范围的屏幕矩形）下方并按 `frame` 重绘。
    fn show(&self, frame: Frame, rect: ScreenRect);

    fn hide(&self);
}

/// 不画候选窗口的空实现。
pub struct NoopSink;

impl CandidateSink for NoopSink {
    fn show(&self, _frame: Frame, _rect: ScreenRect) {}

    fn hide(&self) {}
}
