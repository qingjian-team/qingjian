/// 一条待写出的词条。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexiconEntry {
    /// 词。
    pub text: String,

    /// 音节，不带声调，ü 写 v。
    pub syllables: Vec<String>,

    /// 词频。
    pub frequency: u32,
}
