use serde::{Deserialize, Serialize};

use super::frame::Frame;
use super::key::KeyOutcome;
use super::session::SessionId;

/// Server 发给 DLL 的消息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerMessage {
    /// 对一次 [`super::ClientMessage::Key`] 的处理结果。
    KeyResult {
        /// 会话标识。
        session: SessionId,

        /// 这次按键吃掉还是放行。
        outcome: KeyOutcome,

        /// 本次要立即上屏的文本（选词 / 空格上屏 / 标点等）；没有则为 `None`。
        commit: Option<String>,

        /// 处理后要绘制的组句状态（preedit + 候选）；空 [`Frame`] 表示收起候选窗口。
        frame: Frame,
    },

    /// 对一次 [`super::ClientMessage::Commit`] 的答复：缓冲区里原样上屏的文本（拼音字母 / 英文模式下敲的字母）；
    /// 没在组句时为 `None`。Server 侧组句已清空，DLL 收到后把文本落进文档并收起组句。
    Committed {
        /// 会话标识。
        session: SessionId,

        /// 要原样上屏的文本。
        text: Option<String>,
    },

    /// 不由按键触发的重绘（云联想补词、本地整句模型重排到达）。
    Update {
        /// 会话标识。
        session: SessionId,

        /// 要重绘的状态。
        frame: Frame,
    },

    /// 对一次 [`super::ClientMessage::SyncMode`] 的答复：状态条上点出来、还没被取走的目标模式。
    ModeSync {
        /// 会话标识。
        session: SessionId,

        /// `Some(true)` 切英文、`Some(false)` 切中文；`None` 没有待处理的切换。
        english: Option<bool>,
    },

    /// 收到「翻译选中文字」快捷键：请 DLL 在读编辑会话里取当前选区，用
    /// [`super::ClientMessage::Selection`] 回。这是对触发快捷键那次 [`super::ClientMessage::Key`] 的应答
    /// （替代常规 [`Self::KeyResult`]）；随后 DLL 发来的 `Selection` 才引出翻译候选帧。
    RequestSelection {
        /// 会话标识。
        session: SessionId,

        /// 请求标识，回时带上。
        request: u64,
    },
}
