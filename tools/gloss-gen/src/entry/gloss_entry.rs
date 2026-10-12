use serde::{Deserialize, Serialize};

use super::JapaneseSense;

/// 一个中文词的释义：词性、英文译词、日文译词、越南文译词、德文译词。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlossEntry {
    /// 中文词。
    pub word: String,

    /// 词性缩写（`n.` / `v.` …，Core `PartOfSpeech` 认得的那套）；模型给不出就没有。
    pub pos: Option<String>,

    /// 英文译词，最多两个，按常用度。
    #[serde(default)]
    pub en: Vec<String>,

    /// 日文译词，最多两个，按常用度。
    #[serde(default)]
    pub ja: Vec<JapaneseSense>,

    /// 越南文译词，最多两个，按常用度。
    #[serde(default)]
    pub vi: Vec<String>,

    /// 德文译词，最多两个，按常用度；名词含定冠词（`die Schule`）。
    #[serde(default)]
    pub de: Vec<String>,
}

impl GlossEntry {
    /// 所有学习语言都没给出译词的条目没有用。
    pub fn is_useful(&self) -> bool {
        !self.en.is_empty() || !self.ja.is_empty() || !self.vi.is_empty() || !self.de.is_empty()
    }
}
