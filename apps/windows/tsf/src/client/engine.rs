use std::io::{Read, Write};

use qingjian_platform::protocol::{
    ClientMessage, Frame, KeyEvent, PROTOCOL_VERSION, ScreenRect, ServerMessage, SessionId,
    read_message, write_message,
};

use super::{KeyReply, KeyResponse};
use crate::error::ClientError;

/// 连 Server 的一个会话客户端，开在一条已连好的双工流上（Windows 下是命名管道，测试里是内存流）。
/// 传输是一问一答；开关会话与通知类消息单向发。
pub struct EngineClient<S> {
    stream: S,

    /// 本会话标识，随每条消息带上。
    session: SessionId,
}

impl<S: Read + Write> EngineClient<S> {
    /// 开一个会话（Server 不回话）。`app` 是宿主应用的 exe 文件名，Server 据此查按应用的设置。
    pub fn open(
        mut stream: S,
        session: SessionId,
        app: Option<String>,
    ) -> Result<Self, ClientError> {
        write_message(
            &mut stream,
            &ClientMessage::OpenSession {
                session,
                app,
                protocol: PROTOCOL_VERSION,
            },
        )?;
        Ok(Self { stream, session })
    }

    pub fn session(&self) -> SessionId {
        self.session
    }

    /// 在一条临时流上通知 Server「本线程切成了别的输入法」（状态条收起）。不开会话、不回话。
    pub fn notify_ime_switched(mut stream: S, session: SessionId) -> Result<(), ClientError> {
        write_message(&mut stream, &ClientMessage::ImeSwitched { session })?;
        Ok(())
    }

    /// 送一个按键等结果。触发「翻译选中文字」快捷键时回 [`KeyReply::NeedSelection`]，
    /// 调用方须读当前选区再用 [`Self::selection`] 回给 Server。
    pub fn key(&mut self, event: KeyEvent) -> Result<KeyReply, ClientError> {
        let message = ClientMessage::Key {
            session: self.session,
            event,
        };
        match self.call(&message)? {
            ServerMessage::KeyResult {
                outcome,
                commit,
                frame,
                ..
            } => Ok(KeyReply::Result(KeyResponse {
                outcome,
                commit,
                frame,
            })),
            ServerMessage::RequestSelection { request, .. } => {
                Ok(KeyReply::NeedSelection { request })
            }
            _ => Err(ClientError::Unexpected("expected key result")),
        }
    }

    /// 把读到的选区发给 Server，等它回翻译候选帧。空选区时 Server 不进入翻译、回空帧。
    pub fn selection(
        &mut self,
        request: u64,
        text: String,
        rect: ScreenRect,
    ) -> Result<KeyResponse, ClientError> {
        let message = ClientMessage::Selection {
            session: self.session,
            request,
            text,
            rect,
        };
        match self.call(&message)? {
            ServerMessage::KeyResult {
                outcome,
                commit,
                frame,
                ..
            } => Ok(KeyResponse {
                outcome,
                commit,
                frame,
            }),
            _ => Err(ClientError::Unexpected("expected key result for selection")),
        }
    }

    /// 组句期间定时拉一次云联想的异步结果，回最新一帧。
    pub fn poll(&mut self) -> Result<Frame, ClientError> {
        match self.call(&ClientMessage::Poll {
            session: self.session,
        })? {
            ServerMessage::Update { frame, .. } => Ok(frame),
            _ => Err(ClientError::Unexpected("expected update for poll")),
        }
    }

    /// 让 Server 清空缓冲，拿回要原样上屏的文本（没在组句时为 `None`）。
    pub fn commit(&mut self) -> Result<Option<String>, ClientError> {
        match self.call(&ClientMessage::Commit {
            session: self.session,
        })? {
            ServerMessage::Committed { text, .. } => Ok(text),
            _ => Err(ClientError::Unexpected("expected committed for commit")),
        }
    }

    /// 报组句范围的屏幕矩形，Server 据此摆候选窗口。不回话。
    pub fn position_candidates(&mut self, rect: ScreenRect) -> Result<(), ClientError> {
        self.send(&ClientMessage::PositionCandidates {
            session: self.session,
            rect,
        })
    }

    /// 问 Server 状态条上有没有点出待处理的目标模式（`Some(english)`）。
    pub fn sync_mode(&mut self) -> Result<Option<bool>, ClientError> {
        match self.call(&ClientMessage::SyncMode {
            session: self.session,
        })? {
            ServerMessage::ModeSync { english, .. } => Ok(english),
            _ => Err(ClientError::Unexpected("expected mode sync")),
        }
    }

    /// 把当前会话的中英模式推给 Server（悬浮状态条）。不回话。
    pub fn mode_changed(&mut self, english: bool) -> Result<(), ClientError> {
        self.send(&ClientMessage::ModeChanged {
            session: self.session,
            english,
        })
    }

    /// 让 Server 收起候选窗口（组句在 DLL 侧结束、Server 无从知晓时用）。不回话。
    pub fn hide_candidates(&mut self) -> Result<(), ClientError> {
        self.send(&ClientMessage::HideCandidates {
            session: self.session,
        })
    }

    /// 关闭会话，释放 Server 侧状态。不回话。
    pub fn close(mut self) -> Result<(), ClientError> {
        self.send(&ClientMessage::CloseSession {
            session: self.session,
        })
    }

    fn send(&mut self, message: &ClientMessage) -> Result<(), ClientError> {
        write_message(&mut self.stream, message)?;
        Ok(())
    }

    /// 一问一答；对端在帧边界关闭算 [`ClientError::Closed`]。
    fn call(&mut self, message: &ClientMessage) -> Result<ServerMessage, ClientError> {
        self.send(message)?;
        read_message(&mut self.stream)?.ok_or(ClientError::Closed)
    }
}
