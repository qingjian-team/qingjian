#[derive(Debug, thiserror::Error)]
pub enum GlossaryError {
    /// 文件读取失败。
    #[error("failed to read glossary: {0}")]
    Io(#[from] std::io::Error),

    /// 某一行不符合 `词\t词性\t译文` 格式。
    #[error("line {line}: {reason}")]
    Line {
        /// 从 1 开始的行号。
        line: usize,

        /// 具体原因。
        reason: String,
    },

    /// `.qj` 容器层的错误。
    #[error("glossary file: {0}")]
    Container(#[from] qingjian_format::FormatError),

    /// `.qj` 分节能读但内容自相矛盾。
    #[error("corrupt glossary file: {0}")]
    Corrupt(&'static str),
}
