//! 面板条目的使用记录：实现放学习 crate（`PanelUsage`），Core 打开面板时读来排「常用」、从面板上屏时记一次。

use super::PanelUse;

/// 面板里各条目用了几次。Core 不碰文件，读写都交给实现。
pub trait PanelHistory: Send {
    /// 记过的全部条目，[`super::common_items`] 拿去排「常用」。
    fn uses(&self) -> &[PanelUse];

    /// 从面板上屏了 `item`，记一次。失败只记日志。
    fn record(&mut self, item: &str);
}

/// 不记，「常用」就是内置的常用表。
#[derive(Debug, Clone, Copy, Default)]
pub struct NoPanelHistory;

impl PanelHistory for NoPanelHistory {
    fn uses(&self) -> &[PanelUse] {
        &[]
    }

    fn record(&mut self, _item: &str) {}
}
