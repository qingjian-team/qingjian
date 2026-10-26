//! 生词本页上对一个词的改动。

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum VocabularyChange {
    /// 标认识或取消。
    Known { word: String, known: bool },

    /// 删掉这个词，之后再看到从零记起。
    Remove { word: String },
}
