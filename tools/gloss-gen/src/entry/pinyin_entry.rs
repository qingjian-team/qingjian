use serde::{Deserialize, Serialize};

/// 一个中文词的拼音标注（多音字按词义定读音），JSONL 里一行一个。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinyinEntry {
    /// 中文词。
    pub word: String,

    /// 每个字一个音节，不带声调，ü 写 v（`chong qing`）。
    pub pinyin: Vec<String>,
}
