use super::StatusView;

/// 状态条输出端。Router 在工人线程上调，窗口在 UI 线程上，故要 `Send`。
pub trait StatusSink: Send {
    fn show_status(&self, view: StatusView);

    fn hide_status(&self);
}

/// 不画状态条的空实现。
pub struct NoopStatusSink;

impl StatusSink for NoopStatusSink {
    fn show_status(&self, _view: StatusView) {}

    fn hide_status(&self) {}
}
