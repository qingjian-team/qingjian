//! 前端这一侧会出的错。

use qingjian_platform::protocol::CodecError;

/// IBus 前端的错误。
///
/// 这里没有「引擎出错」这一类：引擎在 Server 进程里，前端只会在连接与协议上出问题。
#[derive(Debug, thiserror::Error)]
pub enum IbusError {
    /// 连不上 Server 的 socket（多半是 Server 没启动）。
    #[error("connect to server at {path}: {source}")]
    Connect {
        /// 试的那条 socket 路径。
        path: String,

        /// 底层的 IO 错误。
        source: std::io::Error,
    },

    /// socket 对端不是同一个用户：不是我们的 Server，不往里发任何东西。
    #[error("server socket is owned by another user")]
    UntrustedPeer,

    /// 收发一条消息时出错（超时也算在内）。
    #[error("server io: {0}")]
    Codec(#[from] CodecError),

    /// Server 关掉了连接。协议里不合法的消息会被 Server 直接断开，所以握手写错也是这一条。
    #[error("server closed the connection")]
    Closed,

    /// 把自己的类型序列化成 JSON 时出错。派生出来的类型不会真出这个错，但不值得为它 unwrap。
    #[error("encode message: {0}")]
    Encode(#[from] serde_json::Error),

    /// Server 回了一条预期之外的消息。
    #[error("unexpected reply from server: {0}")]
    Unexpected(String),
}
