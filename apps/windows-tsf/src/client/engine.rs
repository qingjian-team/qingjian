use std::io::{Read, Write};

use qingjian_platform::protocol::{
    ClientMessage, KeyEvent, ServerMessage, SessionId, read_message, write_message,
};

use super::response::KeyResponse;
use crate::error::ClientError;

/// 连 Server 的一个会话客户端。开在一条已连好的双工流上（`cfg(windows)` 下是命名管道，测试里是内存流），
/// 把 TSF 的按键 / 焦点事件编排成 [`ClientMessage`]，把 Server 回的 [`ServerMessage`] 交回给 COM 层去画。
///
/// 只有 [`Self::key`] 需要等应答（[`ServerMessage::KeyResult`]）；开 / 结束组句 / 关会话都是单向发。
/// Server 不由按键触发的重绘（[`ServerMessage::Update`]）与索取上下文（`RequestSurrounding`）待接
/// （需要一个独立读循环，C 步之后再补）。
pub struct EngineClient<S> {
    /// 与 Server 的双工连接。
    stream: S,

    /// 本会话标识，随每条消息带上。
    session: SessionId,
}

impl<S: Read + Write> EngineClient<S> {
    /// 在一条已连上的双工流上开一个会话（发 [`ClientMessage::OpenSession`]，Server 不回话）。
    pub fn open(mut stream: S, session: SessionId) -> Result<Self, ClientError> {
        write_message(&mut stream, &ClientMessage::OpenSession { session })?;
        Ok(Self { stream, session })
    }

    /// 本会话标识。
    pub fn session(&self) -> SessionId {
        self.session
    }

    /// 送一个按键，等 Server 回 [`KeyResponse`]。
    pub fn key(&mut self, event: KeyEvent) -> Result<KeyResponse, ClientError> {
        write_message(
            &mut self.stream,
            &ClientMessage::Key {
                session: self.session,
                event,
            },
        )?;
        match read_message::<_, ServerMessage>(&mut self.stream)? {
            Some(ServerMessage::KeyResult {
                outcome,
                commit,
                frame,
                ..
            }) => Ok(KeyResponse {
                outcome,
                commit,
                frame,
            }),
            Some(ServerMessage::Update { .. }) => {
                Err(ClientError::Unexpected("update before key result"))
            }
            Some(ServerMessage::RequestSurrounding { .. }) => Err(ClientError::Unexpected(
                "surrounding request before key result",
            )),
            None => Err(ClientError::Closed),
        }
    }

    /// 焦点离开 / 文档要求结束组句：让 Server 把缓冲区上屏并清空（[`ClientMessage::Commit`]，不回话）。
    pub fn commit(&mut self) -> Result<(), ClientError> {
        write_message(
            &mut self.stream,
            &ClientMessage::Commit {
                session: self.session,
            },
        )?;
        Ok(())
    }

    /// 关闭会话，释放 Server 侧状态（[`ClientMessage::CloseSession`]，不回话），消费自身。
    pub fn close(mut self) -> Result<(), ClientError> {
        write_message(
            &mut self.stream,
            &ClientMessage::CloseSession {
                session: self.session,
            },
        )?;
        Ok(())
    }
}
