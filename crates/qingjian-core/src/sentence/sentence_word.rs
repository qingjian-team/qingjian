/// 整句路径上的一个词。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentenceWord {
    /// 词。
    pub text: String,

    /// 词的全拼音节。
    pub syllables: Vec<String>,

    /// 是占位音节（词库里连单字都没有，用拼音本身顶着），不是词库里的词。
    pub placeholder: bool,
}
