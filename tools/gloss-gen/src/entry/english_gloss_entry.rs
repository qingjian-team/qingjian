use serde::{Deserialize, Serialize};

/// 一个英文词的中文释义（英文候选右侧显示），JSONL 里一行一个。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnglishGlossEntry {
    /// 英文词，按词表里的写法。
    pub word: String,

    /// 词性缩写（Core `PartOfSpeech` 认得的那套）；模型给不出就没有。
    pub pos: Option<String>,

    /// 中文释义，最多两个，按常用度。
    pub zh: Vec<String>,
}
