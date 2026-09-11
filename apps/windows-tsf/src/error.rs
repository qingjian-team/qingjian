//! DLL 引擎层（管道客户端）的错误。

use qingjian_platform::protocol::CodecError;

/// 与 Server 通信时的错误。
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// 帧编解码 / 底层 IO 出错。
    #[error("codec: {0}")]
    Codec(#[from] CodecError),

    /// 等应答时对端在帧边界关闭了连接。
    #[error("server closed the connection")]
    Closed,

    /// 收到了与当前请求不匹配的消息（协议错乱）。
    #[error("unexpected server message: {0}")]
    Unexpected(&'static str),
}
