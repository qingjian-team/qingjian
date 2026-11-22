use serde::{Deserialize, Serialize};

/// 模型给一个主题列出的词（JSONL 里一行一个主题）：词、读音、相对常用度。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicVocab {
    /// 问到的那条主题（或提示词），也是 JSONL 的续跑键。
    pub topic: String,

    /// 这个主题下列出的词。
    #[serde(default)]
    pub words: Vec<VocabWord>,
}

/// 一个候选词。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VocabWord {
    /// 中文词。
    pub word: String,

    /// 每个字一个音节，不带声调，ü 写 v（`nv3` → `nv`）。
    #[serde(default)]
    pub pinyin: Vec<String>,

    /// 模型给的相对常用度（1–10000）。没给这条就不收词 —— 词频是数据的一部分，不替它编。
    pub count: u32,
}

impl VocabWord {
    /// 词库 TSV 的第二列：音节空格分隔。
    pub fn key(&self) -> String {
        self.pinyin.join(" ")
    }
}
