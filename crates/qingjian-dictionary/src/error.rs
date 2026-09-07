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
}
