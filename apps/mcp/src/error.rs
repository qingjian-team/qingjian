//! 统一错误：I/O / JSON / 数据加载 / 工具调用 / 协议。MCP 协议错误在 [`protocol`] 里按 JSON-RPC 码映射。

use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O: {0}")]
    Io(#[from] io::Error),

    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("词库: {0}")]
    Dictionary(#[from] qingjian_dictionary::DictionaryError),

    #[error("释义表: {0}")]
    Glossary(#[from] qingjian_translate::GlossaryError),

    #[error("引擎: {0}")]
    Engine(String),

    #[error("工具: {0}")]
    Tool(String),

    #[error("协议: {0}")]
    Protocol(String),
}
