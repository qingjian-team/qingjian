use super::SentenceWord;

/// 一次整句转换的结果。
#[derive(Debug, Clone, PartialEq)]
pub struct Conversion {
    /// 整句文本。
    pub text: String,

    /// 覆盖的全拼音节，与 `text` 逐字对应。
    pub syllables: Vec<String>,

    /// 路径上的词，按顺序。只有一个说明整段本来就是一个词，不必当整句显示。
    pub words: Vec<SentenceWord>,

    /// 路径得分（log 概率之和），越大越好。
    pub score: f64,

    /// 路径上模糊音 / 敲错变体的代价之和（已含在 `score` 里）：大于 0 说明这条路径不是按敲的原样读的。
    pub penalty: f64,
}

impl Conversion {
    pub fn word_count(&self) -> usize {
        self.words.len()
    }

    /// 路径里有音节不是按敲的原样读的（模糊音或敲错变体）。
    pub fn altered(&self) -> bool {
        self.penalty > 0.0
    }

    /// 路径里有占位音节（没转成字的拼音）。
    pub fn has_placeholder(&self) -> bool {
        self.words.iter().any(|w| w.placeholder)
    }
}
