#[derive(Debug, thiserror::Error)]
pub enum DictionaryError {
    /// 文件读取失败。
    #[error("failed to read dictionary: {0}")]
    Io(#[from] std::io::Error),

    /// 某一行不符合 `词\t音节\t词频` 格式。
    #[error("line {line}: {reason}")]
    Line {
        /// 从 1 开始的行号。
        line: usize,

        /// 具体原因。
        reason: &'static str,
    },

    /// `.qj` 容器层的错误（魔数、版本、分节缺失）。
    #[error("dictionary file: {0}")]
    Format(#[from] qingjian_format::FormatError),

    /// `.qj` 分节能读但内容自相矛盾（偏移越界）。
    #[error("corrupt dictionary file: {0}")]
    Corrupt(&'static str),

    /// 码表里的码不是 `a-z`、或长度不在 1–8。
    #[error("invalid code: {0:?}")]
    InvalidCode(String),

    /// 一份码表里一条可用的码都没有（不是码表文件，或码列全非法）。
    #[error("no usable code entries in this file")]
    NoCodeEntries,

    /// 要导入成拼音词库的文件第二列是形码编码（五笔词库），见 [`crate::import::import_code_words`]。
    #[error("the second column holds shape codes, not pinyin")]
    CodeWords,

    /// 方案的词码规则写错了（词长不到 2，或公式不是「大写选字 + 小写选码位」成对写）。
    #[error("invalid word rule: {0}")]
    InvalidWordRule(String),

    /// 一份无读音词表里一条词都没派生出读音（空词表，或词里的字参考词库都不认识）。
    #[error("no derivable words in this word list")]
    NoWordEntries,
}
