use serde::{Deserialize, Serialize};

use super::key::KeyEvent;
use super::session::SessionId;

/// DLL（客户端，每个应用进程里一个）发给 Server 的消息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientMessage {
    /// 进入一个 TSF 文档，开一个会话。
    OpenSession {
        /// 会话标识。
        session: SessionId,
    },

    /// 一次按键，等 Server 回 [`super::ServerMessage::KeyResult`]。
    Key {
        /// 会话标识。
        session: SessionId,

        /// 按键内容。
        event: KeyEvent,
    },

    /// 焦点离开 / 文档要求结束组句：Server 应把缓冲区里的内容上屏并清空。
    Commit {
        /// 会话标识。
        session: SessionId,
    },

    /// 回应 [`super::ServerMessage::RequestSurrounding`]：应用光标前的一段文本，供整句前文用。
    Surrounding {
        /// 会话标识。
        session: SessionId,

        /// 请求标识，对上是哪一次询问。
        request: u64,

        /// 光标前最多约 64 字的上下文；取不到时为空串。
        text: String,
    },

    /// 关闭会话，释放 Server 侧状态。
    CloseSession {
        /// 会话标识。
        session: SessionId,
    },
}
