//! 本机模型服务用不了的原因：`#[error]` 的英文给日志，各壳按变体换成界面语言。

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LocalServerError {
    /// 配置里指定的路径没有这个程序，或者没指定时 PATH 与常见安装位置里都找不到。
    #[error("llama-server not found: set `local_server` or add its folder to PATH")]
    ServerNotFound,

    /// 模型文件不存在（路径照配置原样）。
    #[error("model file not found: {0}")]
    ModelNotFound(String),

    /// 端口被别的程序占着（不是这个模型的 llama-server）。
    #[error("port {0} is taken by another program")]
    PortTaken(u16),

    /// 程序起不来（没有执行权限、不是可执行文件等），附系统的报错。
    #[error("cannot start llama-server: {0}")]
    Spawn(String),

    /// 起来后又退出了，附它输出里最后一条报错。
    #[error("llama-server exited: {0}")]
    Exited(String),

    /// 等到时限还在加载模型。
    #[error("the model is still loading")]
    Loading,
}
