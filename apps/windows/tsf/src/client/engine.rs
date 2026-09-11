use std::io::{Read, Write};

use qingjian_platform::protocol::{
    ClientMessage, Frame, KeyEvent, ServerMessage, SessionId, read_message, write_message,
};

use super::response::KeyResponse;
use crate::error::ClientError;

/// 连 Server 的一个会话客户端，开在一条已连好的双工流上（Windows 下是命名管道，测试里是内存流）。
/// 传输是一问一答：[`Self::key`] 与 [`Self::poll`] 等应答，其余单向发。
pub struct EngineClient<S> {
    /// 与 Server 的双工连接。
    stream: S,

    /// 本会话标识，随每条消息带上。
    session: SessionId,
}

impl<S: Read + Write> EngineClient<S> {
    /// 开一个会话（Server 不回话）。
    pub fn open(mut stream: S, session: SessionId) -> Result<Self, ClientError> {
        write_message(&mut stream, &ClientMessage::OpenSession { session })?;
        Ok(Self { stream, session })
    }

    pub fn session(&self) -> SessionId {
        self.session
    }

    /// 送一个按键，等 Server 回结果。
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
            Some(_) => Err(ClientError::Unexpected("expected key result")),
            None => Err(ClientError::Closed),
        }
    }

    /// 组句期间定时拉一次云联想的异步结果，回最新一帧。
    pub fn poll(&mut self) -> Result<Frame, ClientError> {
        write_message(
            &mut self.stream,
            &ClientMessage::Poll {
                session: self.session,
            },
        )?;
        match read_message::<_, ServerMessage>(&mut self.stream)? {
            Some(ServerMessage::Update { frame, .. }) => Ok(frame),
            Some(_) => Err(ClientError::Unexpected("expected update for poll")),
            None => Err(ClientError::Closed),
        }
    }

    /// 焦点离开 / 文档要求结束组句：让 Server 清空缓冲（不回话）。
    pub fn commit(&mut self) -> Result<(), ClientError> {
        write_message(
            &mut self.stream,
            &ClientMessage::Commit {
                session: self.session,
            },
        )?;
        Ok(())
    }

    /// 关闭会话，释放 Server 侧状态（不回话）。
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
