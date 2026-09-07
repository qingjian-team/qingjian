use thiserror::Error;

#[derive(Debug, Error)]
pub enum LmError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("bad line {line} in {file}: {reason}")]
    Format {
        /// 文件名。
        file: &'static str,

        /// 行号（从 1 数）。
        line: usize,

        /// 出错原因。
        reason: &'static str,
    },

    #[error("language model file: {0}")]
    Container(#[from] qingjian_format::FormatError),

    #[error("corrupt language model file: {0}")]
    Corrupt(&'static str),
}
