//! 几个输入上下文共用的那一条连接，外加边界上的毒化恢复与重连。
//!
//! 这是 IBus 走进程内引擎那一版里 `Shared`（`Arc<Mutex<Router>>`）的位置，
//! 换成了连接加会话表：引擎搬到 Server 进程之后，前端这边要守的不再是引擎状态，
//! 而是**一条会断的连接**，以及「哪个输入上下文对应 Server 上的哪个会话」。
//!
//! 三件事在这里收口：
//!
//! - **锁不会被毒化拖死。** panic 时 `Mutex` 会被标记成 poisoned，之后每次 `lock()` 都返回错误；
//!   照 `expect` 写下去等于「崩过一次就再也打不了字」。一律取回里面的数据继续用。
//! - **连接断了自己接回来。** Server 重启、超时、协议出错都把连接扔掉，下一次按键重连；
//!   会话跟着连接作废，重连后按同一个上下文标识重开，调用方不用管。
//! - **隐私状态跟着会话走。** 能力（密码框 / 私密输入）记在这里，重开会话时**先补报能力再干别的**：
//!   Server 在收到能力之前不接受按键，而重连恰恰发生在用户正在打字的时候。

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use qingjian_platform::protocol::SessionId;
use qingjian_platform::protocol::linux::Capabilities;

use super::Connection;
use crate::error::IbusError;

/// 共用的连接句柄。
#[derive(Clone, Default)]
pub struct Shared {
    /// 连接与会话表。
    state: Arc<Mutex<State>>,
}

/// 锁里面的东西。
#[derive(Default)]
struct State {
    /// 进程内唯一的连接；还没连上或刚断开时是 `None`。
    connection: Option<Connection>,

    /// 上下文标识 → Server 上的会话编号。连接一断整张表作废。
    sessions: HashMap<String, SessionId>,

    /// 上下文标识 → 最近一次报给 Server 的能力。**跨重连保留**，重开会话时照着补报。
    capabilities: HashMap<String, Capabilities>,
}

impl Shared {
    /// 在某个输入上下文的会话上做一次事情。连接或会话不在就先建。
    ///
    /// **出错就扔掉连接**：一问一答的协议上，一次失败之后收发就可能串位，与其猜不如重来。
    /// 会话表跟着清空，下一次调用重开。
    pub fn with_session<T>(
        &self,
        context: &str,
        action: impl FnOnce(&mut Connection, SessionId) -> Result<T, IbusError>,
    ) -> Result<T, IbusError> {
        let mut state = self.lock();
        match Self::ensure_session(&mut state, context) {
            Ok(session) => {
                let connection = state.connection.as_mut().expect("会话在就有连接");
                match action(connection, session) {
                    Ok(value) => Ok(value),
                    Err(error) => Err(Self::drop_connection(&mut state, error)),
                }
            }
            Err(error) => Err(Self::drop_connection(&mut state, error)),
        }
    }

    /// 记下某个上下文的能力并立刻报给 Server。
    ///
    /// 值没变就什么都不做：`SetContentType` 在有些应用里每次聚焦都来一遍，照发会平白多一轮往返。
    /// 返回 `Ok(true)` 表示确实报了一次。
    pub fn set_capabilities(
        &self,
        context: &str,
        capabilities: Capabilities,
    ) -> Result<bool, IbusError> {
        {
            let mut state = self.lock();
            if state.capabilities.get(context) == Some(&capabilities) {
                return Ok(false);
            }
            state.capabilities.insert(context.to_owned(), capabilities);
        }
        // 会话还没开的话这里顺带把它开起来，开的时候就会带上刚记下的能力
        self.with_session(context, |connection, session| {
            connection.capabilities(session, capabilities)
        })?;
        Ok(true)
    }

    /// 关掉某个上下文的会话（IBus 的引擎对象销毁时）。连接已经断了就只清表。
    pub fn close(&self, context: &str) {
        let mut state = self.lock();
        state.capabilities.remove(context);
        let Some(session) = state.sessions.remove(context) else {
            return;
        };
        if let Some(connection) = state.connection.as_mut()
            && let Err(error) = connection.close_session(session)
        {
            tracing::debug!(%error, "关会话没送出去，连接多半已经断了");
            state.connection = None;
            state.sessions.clear();
        }
    }

    /// 现在连着吗（`--check` 与日志用）。
    pub fn connected(&self) -> bool {
        self.lock().connection.is_some()
    }

    /// 连接与会话都就位，返回会话编号。
    fn ensure_session(state: &mut State, context: &str) -> Result<SessionId, IbusError> {
        if state.connection.is_none() {
            state.connection = Some(Connection::connect()?);
            // 连接换了，旧会话编号在新连接上没有意义
            state.sessions.clear();
        }
        if let Some(session) = state.sessions.get(context) {
            return Ok(*session);
        }
        let connection = state.connection.as_mut().expect("刚连上");
        let session = connection.open_session(context)?;
        // 能力必须排在按键前面：Server 在收到它之前不收按键
        let capabilities = state.capabilities.get(context).copied().unwrap_or_default();
        connection.capabilities(session, capabilities)?;
        state.sessions.insert(context.to_owned(), session);
        Ok(session)
    }

    /// 出错之后把连接与会话表一起扔掉，错误原样交出去。
    fn drop_connection(state: &mut State, error: IbusError) -> IbusError {
        tracing::warn!(%error, "与 Server 的连接出错，丢弃后下次重连");
        state.connection = None;
        state.sessions.clear();
        error
    }

    /// 拿锁。被毒化过就把里面的数据取回来接着用，不把整个输入法拖死。
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|poisoned| {
            tracing::error!("连接的锁被上一次 panic 毒化过，取回接着用");
            poisoned.into_inner()
        })
    }
}
