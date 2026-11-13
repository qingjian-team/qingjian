//! 一条连接上开过的会话：DLL 报的会话号只在本连接内有效，Router 里用的是 Server 分配的全局号。
//!
//! DLL 拿线程 id 当会话号，别的进程枚举得到；只按这个号寻址的话，任何连上管道的进程都能
//! `Poll` 别的应用的组句与候选（#699）。按连接翻译之后，一条连接只碰得到自己开过的会话；
//! 号也不会和别的连接撞（断线重连、线程 id 被系统复用都没关系）。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use qingjian_platform::protocol::{ClientMessage, ServerMessage, SessionId};

use super::Inbound;

/// 全局会话号，从 1 起；0 留给 [`NO_SESSION`]。
static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

/// 不属于任何会话的消息（临时连接上的 `ImeSwitched`）交给 Router 时用的号。
const NO_SESSION: SessionId = SessionId(0);

/// 本连接的会话号对照：DLL 报的号 → 全局号。
#[derive(Default)]
pub struct ConnectionSessions {
    open: HashMap<SessionId, SessionId>,
}

impl ConnectionSessions {
    /// 把 DLL 报的会话号换成全局号。
    pub fn inbound(&mut self, mut message: ClientMessage) -> Inbound {
        let local = *message.session_mut();
        let global = match &message {
            ClientMessage::OpenSession { .. } => {
                if self.open.contains_key(&local) {
                    return Inbound::Reject;
                }
                let global = SessionId(NEXT_SESSION.fetch_add(1, Ordering::Relaxed));
                self.open.insert(local, global);
                global
            }
            ClientMessage::CloseSession { .. } => match self.open.remove(&local) {
                Some(global) => global,
                None => return Inbound::Ignore,
            },
            // DLL 被停用后用一条临时连接发，不开会话；Router 只收状态条，不看会话号。
            ClientMessage::ImeSwitched { .. } => {
                self.open.get(&local).copied().unwrap_or(NO_SESSION)
            }
            _ => match self.open.get(&local) {
                Some(&global) => global,
                None => return Inbound::Reject,
            },
        };
        *message.session_mut() = global;
        Inbound::Forward { message, local }
    }

    /// 回包里的全局号换回 DLL 报的号。
    pub fn outbound(mut response: ServerMessage, local: SessionId) -> ServerMessage {
        *response.session_mut() = local;
        response
    }

    /// 还开着的会话的全局号：连接断了要照单关掉。
    pub fn drain(&mut self) -> impl Iterator<Item = SessionId> + '_ {
        self.open.drain().map(|(_, global)| global)
    }
}
