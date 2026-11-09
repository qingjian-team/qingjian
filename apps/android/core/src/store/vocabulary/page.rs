//! 生词本页的内容。

use serde::Serialize;

use super::VocabularyWord;

/// 生词本页：几个汇总数字、这门语言的等级名、筛出来的词条（最近见到的在前）。
#[derive(Debug, Default, Serialize)]
pub struct VocabularyPage {
    /// 见过的译词数。
    pub seen: u64,

    /// 已经眼熟（见得够多或标了认识）的。
    pub familiar: u64,

    /// 最近 7 天新见的。
    pub new_this_week: u64,

    /// 等级从易到难（`A1`…`C2`）；这门语言没有等级表为空。
    pub levels: Vec<String>,

    /// 符合筛选的词数；`words` 最多给 `PAGE_LIMIT` 条。
    pub matched: u64,

    pub words: Vec<VocabularyWord>,
}
