//! 生词本的两个视图。

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    /// 按天的时间线：每次相遇一条。
    #[default]
    Timeline,
    /// 按词的表：筛选、标认识、导出。
    Words,
}

impl View {
    pub const ALL: [Self; 2] = [Self::Timeline, Self::Words];

    pub fn label(self) -> &'static str {
        match self {
            Self::Timeline => "时间线",
            Self::Words => "词表",
        }
    }
}
