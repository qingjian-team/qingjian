//! librime 初始化失败时给出明确错误，不静默退回青简方案。
#[derive(Debug, thiserror::Error)]
pub enum RimeError {
    #[cfg(not(windows))]
    #[error("Rime library: {0}")]
    Library(#[from] libloading::Error),
    #[error("Rime data directory: {0}")]
    Io(#[from] std::io::Error),
    #[error("Rime path is not UTF-8 or contains NUL")]
    InvalidPath,
    #[error("Rime API lacks {0}; install a current librime with Lua")]
    Api(&'static str),
    #[error("Rime Lua module is unavailable")]
    LuaMissing,
    #[error("Rime module is not registered: {0}")]
    ModuleMissing(String),
    #[error("another Rime runtime is already using different directories")]
    RuntimeConflict,
    #[error("Rime deployment failed")]
    Deployment,
    #[error("Rime session could not be created")]
    Session,
    #[error("Rime schema is unavailable: {0}")]
    Schema(String),
}
