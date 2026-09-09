use std::collections::HashMap;

use qingjian_platform::protocol::{ClientMessage, Frame, KeyOutcome, ServerMessage, SessionId};

use crate::session::Session;

/// 把 DLL 发来的 [`ClientMessage`] 分派到对应会话，产出要回给 DLL 的 [`ServerMessage`]。
///
/// 现在是骨架：会话的开 / 关已成形，按键与上屏还没接 [`qingjian_core::Engine`]，
/// 一律回放行、空帧。接 Engine 后每个会话各自持有组句状态（见 [`Session`]）。
#[derive(Default)]
pub struct Router {
    /// 活跃会话，按 [`SessionId`] 索引。
    sessions: HashMap<SessionId, Session>,
}

impl Router {
    pub fn new() -> Self {
        Self::default()
    }

    /// 处理一条消息；返回 `None` 表示不用回话。
    pub fn handle(&mut self, message: ClientMessage) -> Option<ServerMessage> {
        match message {
            ClientMessage::OpenSession { session } => {
                let state = Session::new(session);
                tracing::debug!(id = ?state.id(), "会话打开");
                self.sessions.insert(session, state);
                None
            }
            ClientMessage::Key { session, event } => {
                // TODO(windows)：喂给该会话的 Engine，回真实的候选与上屏文本。
                tracing::trace!(?session, ?event, "按键（待接 Engine）");
                Some(ServerMessage::KeyResult {
                    session,
                    outcome: KeyOutcome::Passthrough,
                    commit: None,
                    frame: Frame::default(),
                })
            }
            ClientMessage::Commit { session } => {
                // TODO(windows)：把缓冲区里的内容上屏并清空。
                tracing::debug!(?session, "上屏并结束组句（待接 Engine）");
                None
            }
            ClientMessage::Surrounding {
                session,
                request,
                text,
            } => {
                // TODO(windows)：把前文交给该会话的 Engine 做整句前文。
                tracing::trace!(
                    ?session,
                    request,
                    chars = text.chars().count(),
                    "收到上下文"
                );
                None
            }
            ClientMessage::CloseSession { session } => {
                self.sessions.remove(&session);
                tracing::debug!(?session, "会话关闭");
                None
            }
        }
    }

    /// 当前活跃会话数。
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
}
