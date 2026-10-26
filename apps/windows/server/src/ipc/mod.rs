//! Server 与 DLL 之间的传输。线上帧格式在 [`qingjian_platform::protocol`]（两端共用），
//! 这层只提供双工字节流上的消息循环（[`serve`]）与具体传输（命名管道 `pipe`）。

mod handled;
#[cfg(windows)]
pub mod pipe;
mod work;

pub use handled::Handled;
pub use qingjian_platform::protocol::{CodecError, read_message, write_message};
pub use work::Work;

use std::io::{Read, Write};

use qingjian_platform::protocol::{ClientMessage, SessionId};

use crate::dispatch::Router;

/// 在一条已连上的双工流上服务一个客户端：读消息、交给 Router、写回，直到对端在帧边界关闭。
pub fn serve<S: Read + Write>(stream: &mut S, router: &mut Router) -> Result<(), CodecError> {
    let mut opened = Vec::new();
    let outcome = pump(
        stream,
        |message| Handled::Reply(router.handle(message).map(Box::new)),
        &mut opened,
    );
    // 对端就这么没了（应用被关 / 崩了 / 管道断）时 DLL 来不及发 `CloseSession`：
    // 会话留在 Router 里，聚焦会话的组句与候选窗口会一直挂着，直到别的应用来一个键把它挤掉。
    for &session in &opened {
        router.handle(ClientMessage::CloseSession { session });
    }
    outcome
}

/// 连接循环：读 → 交给 `dispatch` → 写回，直到对端在帧边界关闭、读写出错或 `dispatch` 要停。
/// 本连接上开过的会话（`OpenSession`）记进 `opened`，调用方在连接断开后照单收掉。
/// 线上的命名管道与测试里的内存流都走这一段，断开后的收尾行为因此不会出现两份。
fn pump<S: Read + Write>(
    stream: &mut S,
    mut dispatch: impl FnMut(ClientMessage) -> Handled,
    opened: &mut Vec<SessionId>,
) -> Result<(), CodecError> {
    while let Some(message) = read_message::<_, ClientMessage>(&mut *stream)? {
        if let ClientMessage::OpenSession { session, .. } = &message {
            opened.push(*session);
        }
        match dispatch(message) {
            Handled::Reply(Some(response)) => write_message(&mut *stream, &*response)?,
            Handled::Reply(None) => {}
            Handled::Stop => break,
        }
    }
    Ok(())
}
