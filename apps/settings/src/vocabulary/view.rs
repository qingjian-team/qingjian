//! 生词本的两个视图。
use crate::i18n::tr;

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
            Self::Timeline => tr("vocabulary.view_timeline"),
            Self::Words => tr("vocabulary.view_words"),
        }
    }
}
