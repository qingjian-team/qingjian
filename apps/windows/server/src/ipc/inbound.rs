//! 连接循环里一条消息经会话号翻译后的去向（见 [`super::sessions`]）。

use qingjian_platform::protocol::{ClientMessage, SessionId};

/// 一条进来的消息该怎么办。
pub enum Inbound {
    /// 换成全局号交给 Router；回包要把号换回 `local`。
    Forward {
        message: ClientMessage,
        local: SessionId,
    },

    /// 关的是本连接没开过的会话（已经收掉了）：不用理，也不回话。
    Ignore,

    /// 对没开过的会话发消息、或重复开同一个会话：不是我们的 DLL 会做的事，断开连接。
    Reject,
}
