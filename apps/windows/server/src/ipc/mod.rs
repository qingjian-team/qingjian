//! Server 与 DLL 之间的传输。线上帧格式在 [`qingjian_platform::protocol`]（两端共用），
//! 这层只提供双工字节流上的消息循环（[`serve`]）与具体传输（命名管道 `pipe`）。

mod handled;
mod inbound;
#[cfg(windows)]
pub mod instance;
#[cfg(windows)]
pub mod pipe;
mod sessions;
mod work;

pub use handled::Handled;
use inbound::Inbound;
pub use qingjian_platform::protocol::{CodecError, read_message, write_message};
pub use work::Work;

use std::io::{Read, Write};

use qingjian_platform::protocol::ClientMessage;

use self::sessions::ConnectionSessions;
use crate::dispatch::Router;

/// 在一条已连上的双工流上服务一个客户端：读消息、交给 Router、写回，直到对端在帧边界关闭。
pub fn serve<S: Read + Write>(stream: &mut S, router: &mut Router) -> Result<(), CodecError> {
    let mut sessions = ConnectionSessions::default();
    let outcome = pump(
        stream,
        |message| Handled::Reply(router.handle(message).map(Box::new)),
        &mut sessions,
    );
    // 对端就这么没了（应用被关 / 崩了 / 管道断）时 DLL 来不及发 `CloseSession`：
    // 会话留在 Router 里，聚焦会话的组句与候选窗口会一直挂着，直到别的应用来一个键把它挤掉。
    for session in sessions.drain() {
        router.handle(ClientMessage::CloseSession { session });
    }
    outcome
}

/// 连接循环：读 → 交给 `dispatch` → 写回，直到对端在帧边界关闭、读写出错、`dispatch` 要停，
/// 或对端碰了本连接没开过的会话。会话号经 `sessions` 在 DLL 的号与全局号之间翻译，
/// 调用方在连接断开后把 `sessions` 里剩下的照单收掉。
/// 线上的命名管道与测试里的内存流都走这一段，断开后的收尾行为因此不会出现两份。
fn pump<S: Read + Write>(
    stream: &mut S,
    mut dispatch: impl FnMut(ClientMessage) -> Handled,
    sessions: &mut ConnectionSessions,
) -> Result<(), CodecError> {
    while let Some(message) = read_message::<_, ClientMessage>(&mut *stream)? {
        let (message, local) = match sessions.inbound(message) {
            Inbound::Forward { message, local } => (message, local),
            Inbound::Ignore => continue,
            Inbound::Reject => {
                tracing::warn!("Client used a session it never opened, disconnecting");
                break;
            }
        };
        match dispatch(message) {
            Handled::Reply(Some(response)) => {
                let response = ConnectionSessions::outbound(*response, local);
                write_message(&mut *stream, &response)?;
            }
            Handled::Reply(None) => {}
            Handled::Stop => break,
        }
    }
    Ok(())
}
