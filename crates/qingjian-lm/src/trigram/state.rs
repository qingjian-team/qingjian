//! 切词束搜索里的一个状态：最近两个词与到这里的累计分。

/// 一条部分路径的末尾：`previous` / `last` 是最近两个词的编号（`None` 是还没有词），`log10` 是累计 log10 P。
#[derive(Debug, Clone, Copy)]
pub(super) struct State {
    pub previous: Option<u32>,

    pub last: Option<u32>,

    pub log10: f64,
}

impl State {
    /// 一段文字的起点。
    pub const START: Self = Self {
        previous: None,
        last: None,
        log10: 0.0,
    };
}
