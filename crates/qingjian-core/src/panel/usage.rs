//! 面板里一个条目被用了几次；落盘在 `qingjian-learning` 的 `PanelUsage`，排「常用」用。

/// 一个条目（符号、emoji，可以是带肤色的）从面板上屏的次数与最后一次的时刻。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelUse {
    pub item: String,

    pub count: u32,

    /// 最后一次用的时刻（Unix 秒），次数一样时近的在前。
    pub last: i64,
}
