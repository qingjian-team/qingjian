//! 与 Server 的一条 Unix socket 连接。
//!
//! 一个进程一条连接，进程里每个 IBus 输入上下文在这条连接上开一个会话
//! （Server 侧按「连接代次 + 上下文」隔离，见 `qingjian_platform::protocol::linux`）。
//!
//! 收发是同步的一问一答，与 Fcitx5 插件的 `ipc/connection.cpp` 同一套：
//! **哪些消息有回包是协议的一部分**——`LinuxEvent` 与 `OpenSession` 有，
//! `CloseSession` 与展示回执没有。读多了会卡住，读少了下一次答复会串位。

use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use qingjian_platform::protocol::linux::{
    Capabilities, LINUX_UI_PROTOCOL, LinuxEvent, socket_path,
};
use qingjian_platform::protocol::{
    ClientMessage, KeyEvent, PROTOCOL_VERSION, SessionId, read_message, write_message,
};
use serde_json::{Value, json};

use super::Reply;
use crate::error::IbusError;

/// 一次收发的截止时间，与 Fcitx5 插件的 200 毫秒一致：输入法卡在 IO 上比丢一次按键更糟，
/// 超时就当连接坏了断开，下一次按键重连。
const TIMEOUT: Duration = Duration::from_millis(200);

/// 连接代次，Server 靠它认出「重连之后的新一批会话」，旧连接残留的展示回执按它作废。
/// Server 要求大于 0，且一条连接上的会话代次必须一致，所以按连接取一次。
static GENERATION: AtomicU64 = AtomicU64::new(1);

/// 一条连接。
pub struct Connection {
    /// socket 本身。
    stream: UnixStream,

    /// 本连接的代次。
    generation: u64,

    /// 下一个会话编号（连接内局部，Server 会映射成全局的）。
    next_session: u64,
}

impl Connection {
    /// 连上 Server。
    ///
    /// 连上之后核对对端是不是同一个用户：socket 在 `$XDG_RUNTIME_DIR` 下本来就是私有的，
    /// 但路径可以被 `QINGJIAN_SOCKET` 换掉，所以与 Server 侧一样两头都查一次。
    pub fn connect() -> Result<Self, IbusError> {
        Self::connect_at(&socket_path())
    }

    /// 连指定路径上的 Server。测试用假 Server 时走这条，不去动进程级的环境变量。
    pub fn connect_at(path: &std::path::Path) -> Result<Self, IbusError> {
        let stream = UnixStream::connect(path).map_err(|source| IbusError::Connect {
            path: path.display().to_string(),
            source,
        })?;
        stream.set_read_timeout(Some(TIMEOUT)).ok();
        stream.set_write_timeout(Some(TIMEOUT)).ok();
        if !same_user(&stream) {
            return Err(IbusError::UntrustedPeer);
        }
        Ok(Self {
            stream,
            generation: GENERATION.fetch_add(1, Ordering::Relaxed),
            next_session: 1,
        })
    }

    /// 给一个 IBus 输入上下文开会话：`OpenSession` 之后立刻握 `LinuxHello`。
    ///
    /// `context` 是这个上下文的标识（Server 要求非空、不超过 64 字节、同连接内不重复），
    /// 用 IBus 的输入上下文对象路径正好。
    pub fn open_session(&mut self, context: &str) -> Result<SessionId, IbusError> {
        let session = SessionId(self.next_session);
        self.next_session += 1;
        // 宿主应用的标识（`[apps]` 按应用设置要用）IBus 这边还没有着落，先不报
        let open = serde_json::to_value(ClientMessage::OpenSession {
            session,
            app: None,
            protocol: PROTOCOL_VERSION,
        })?;
        self.request(&open)?;
        let hello = json!({"LinuxHello": {
            "session": session,
            "context": context,
            "generation": self.generation,
            "version": LINUX_UI_PROTOCOL,
        }});
        let reply = self.request(&hello)?;
        if reply.get("LinuxHello").is_none() {
            return Err(IbusError::Unexpected(reply.to_string()));
        }
        Ok(session)
    }

    /// 报一次框架能力（这个输入框是不是密码框之类）。
    ///
    /// **必须在别的事件之前报**：Server 在收到它之前不接受按键（没有它就不知道该不该进隐私模式）。
    pub fn capabilities(
        &mut self,
        session: SessionId,
        capabilities: Capabilities,
    ) -> Result<Reply, IbusError> {
        self.event(session, LinuxEvent::Capabilities(capabilities))
    }

    /// 报一次按键。`release` 是抬键——中 / 英切换要认 Shift 的抬起，所以抬键也得报。
    pub fn key(
        &mut self,
        session: SessionId,
        event: KeyEvent,
        release: bool,
    ) -> Result<Reply, IbusError> {
        self.event(session, LinuxEvent::Key { event, release })
    }

    /// 报一次焦点进出。
    pub fn focus(&mut self, session: SessionId, focused: bool) -> Result<Reply, IbusError> {
        self.event(session, LinuxEvent::Focus { focused })
    }

    /// 这个上下文不再活跃（焦点离开、切走输入法、能力变了）。
    ///
    /// 缓冲里的东西怎么处置由 Server 按这三个事实决定：`focus_out` 且 `client_preedit` 时不上屏
    /// （应用自己画着 preedit 的场合），`capability_changed` 时直接丢弃（进了密码框就不能留着）。
    pub fn deactivate(
        &mut self,
        session: SessionId,
        focus_out: bool,
        client_preedit: bool,
        capability_changed: bool,
    ) -> Result<Reply, IbusError> {
        self.event(
            session,
            LinuxEvent::Deactivate {
                focus_out,
                client_preedit,
                capability_changed,
            },
        )
    }

    /// 应用要求重置：丢掉组句，不上屏。
    pub fn reset(&mut self, session: SessionId) -> Result<Reply, IbusError> {
        self.event(session, LinuxEvent::Reset)
    }

    /// 组句期间取一次最新的帧：本地整句模型的重排结果从这里来（Server 不推送）。
    /// 回的是 `Update`，帧没变时版本号与上次相同。
    pub fn poll(&mut self, session: SessionId) -> Result<Reply, IbusError> {
        let message = serde_json::to_value(ClientMessage::Poll { session })?;
        Reply::from_value(self.request(&message)?)
    }

    /// 关掉一个会话。没有回包。
    pub fn close_session(&mut self, session: SessionId) -> Result<(), IbusError> {
        let message = serde_json::to_value(ClientMessage::CloseSession { session })?;
        self.notify(&message)
    }

    /// 发一个 Linux 事件，取回 Server 的处置。
    fn event(&mut self, session: SessionId, event: LinuxEvent) -> Result<Reply, IbusError> {
        let message = json!({"LinuxEvent": {"session": session, "event": event}});
        Reply::from_value(self.request(&message)?)
    }

    /// 发一条有回包的消息。
    fn request(&mut self, message: &Value) -> Result<Value, IbusError> {
        self.notify(message)?;
        read_message::<_, Value>(&mut self.stream)?.ok_or(IbusError::Closed)
    }

    /// 发一条没有回包的消息。
    fn notify(&mut self, message: &Value) -> Result<(), IbusError> {
        write_message(&mut self.stream, message)?;
        Ok(())
    }
}

/// socket 对端是不是当前用户。
fn same_user(stream: &UnixStream) -> bool {
    let mut credentials = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut size = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: fd 来自还活着的 UnixStream，出参是本地的 ucred 与它的长度
    unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut credentials as *mut libc::ucred).cast(),
            &mut size,
        ) == 0
            && credentials.uid == libc::geteuid()
    }
}
