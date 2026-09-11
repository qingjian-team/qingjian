//! Server 与 DLL 之间的传输。线上帧格式在 [`qingjian_platform::protocol`]（两端共用），
//! 这层只提供双工字节流上的消息循环（[`serve`]）与具体传输（命名管道 [`pipe`]）。

#[cfg(windows)]
pub mod pipe;

pub use qingjian_platform::protocol::{CodecError, read_message, write_message};

use std::io::{Read, Write};

use qingjian_platform::protocol::ClientMessage;

use crate::dispatch::Router;

/// 在一条已连上的双工流上服务一个客户端：读消息、交给 Router、写回，直到对端在帧边界关闭。
pub fn serve<S: Read + Write>(stream: &mut S, router: &mut Router) -> Result<(), CodecError> {
    while let Some(message) = read_message::<_, ClientMessage>(&mut *stream)? {
        if let Some(response) = router.handle(message) {
            write_message(&mut *stream, &response)?;
        }
    }
    Ok(())
}
