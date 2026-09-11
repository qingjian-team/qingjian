//! Server 与 DLL 之间的传输：线上帧格式来自 [`qingjian_platform::protocol`]（Server 与 DLL 共用，
//! 见那里的 `codec`），这层只提供在双工字节流上跑的消息循环（[`serve`]）与具体传输（命名管道 [`pipe`]）。
//!
//! 具体传输把一条连好的**双工**流（命名管道读写同一句柄）交给 [`serve`]，
//! 这层只管「读一条 [`ClientMessage`] → 交给 [`Router`] → 写回 [`ServerMessage`]」。

#[cfg(windows)]
pub mod pipe;

pub use qingjian_platform::protocol::{CodecError, read_message, write_message};

use std::io::{Read, Write};

use qingjian_platform::protocol::ClientMessage;

use crate::dispatch::Router;

/// 在一条已连上的双工字节流上服务一个 DLL 客户端：循环读消息、交给 Router、把回复写回，
/// 直到对端在帧边界关闭（读到干净 EOF）。命名管道 / TCP 等把连好的流交给它。
pub fn serve<S: Read + Write>(stream: &mut S, router: &mut Router) -> Result<(), CodecError> {
    while let Some(message) = read_message::<_, ClientMessage>(&mut *stream)? {
        if let Some(response) = router.handle(message) {
            write_message(&mut *stream, &response)?;
        }
    }
    Ok(())
}
