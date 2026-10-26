//! 生词本页交过来的请求：看哪门语言、怎么筛，顺带一个改动。

use serde::Deserialize;

use super::{VocabularyChange, VocabularyFilter};

/// `{"language":"en","filter":"fresh","level":"B1","search":"dev","change":{"op":"known","word":"develop","known":true}}`
#[derive(Debug, Deserialize)]
pub struct VocabularyRequest {
    /// 学习语言的代码，同键盘设置。
    pub language: String,

    #[serde(default)]
    pub filter: VocabularyFilter,

    /// 只看这一级（`A1`、`N5`）；空串不按级筛。
    #[serde(default)]
    pub level: String,

    /// 搜索：不分大小写，词、带它上屏的中文、例句里有就算（同 PC 生词本）；空串不搜。
    #[serde(default)]
    pub search: String,

    /// 先做这个改动再列。
    #[serde(default)]
    pub change: Option<VocabularyChange>,
}
