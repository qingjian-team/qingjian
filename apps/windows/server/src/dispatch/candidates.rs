//! 候选窗口的输出端：[`Router`](super::Router) 只管产出 [`Frame`] 与光标矩形，怎么画交给这个 trait 的实现。
//!
//! Windows 上由 [`crate::ui::CandidateUi`] 实现，把命令 marshal 到自绘候选窗口的 UI 线程；非 Windows（装配测试）
//! 用 [`NoopSink`] 空实现。候选窗口搬出应用进程、由 Server 自绘，才能盖过微软商店 / 任务栏搜索这些高 z-band 宿主。

use qingjian_platform::protocol::{Frame, ScreenRect};

/// Server 侧候选窗口的抽象输出端。实现要能跨线程持有（Router 在工人线程上调，窗口在 UI 线程上）。
pub trait CandidateSink: Send {
    /// 把候选窗口摆到 `rect`（组句范围的屏幕矩形）下方并按 `frame` 重绘。空帧应收起窗口。
    fn show(&self, frame: Frame, rect: ScreenRect);

    /// 收起候选窗口（组句结束 / 失焦 / 切走会话）。
    fn hide(&self);
}

/// 不画候选窗口的空实现（非 Windows 装配测试，或 UI 线程启动失败时的兜底）。
pub struct NoopSink;

impl CandidateSink for NoopSink {
    fn show(&self, _frame: Frame, _rect: ScreenRect) {}

    fn hide(&self) {}
}
