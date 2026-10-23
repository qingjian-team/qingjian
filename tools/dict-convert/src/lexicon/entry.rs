/// 一条待写出的词条。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexiconEntry {
    /// 词。
    pub text: String,

    /// 音节，不带声调，ü 写 v。
    pub syllables: Vec<String>,

    /// 词频。
    pub frequency: u32,

    /// 逐音节声调（1–5，0 是没标）；空的是整条都不知道。只有通用大词库带来的有。
    pub tones: Vec<u8>,
}
