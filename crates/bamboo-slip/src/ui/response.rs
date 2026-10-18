//! 处理一个事件的结果。

pub struct Response<M> {
    /// 按产生的先后交给应用的 `update`。
    pub messages: Vec<M>,

    /// 交互状态（悬停、按下、焦点）变了，要按新状态重建。
    pub changed: bool,
}

impl<M> Response<M> {
    pub(super) fn new() -> Self {
        Self {
            messages: Vec::new(),
            changed: false,
        }
    }

    /// 有消息或状态变了：更新完应用状态后重建、重画。
    pub fn needs_rebuild(&self) -> bool {
        self.changed || !self.messages.is_empty()
    }
}
