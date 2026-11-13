//! `cfg(windows)`：打开 Server 的命名管道，得到一条双工流交给 [`EngineClient`](super::EngineClient)。
//!
//! 管道名按用户与登录会话算（[`qingjian_platform::pipe`]）；连上先认对面是不是我们的 Server，
//! 认不出来按连不上处理（#699）：抢先占了管道名的别的进程拿不到按键与光标前文。

use std::fs::{File, OpenOptions};
use std::io;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;

use windows::Win32::Foundation::{ERROR_PIPE_BUSY, HANDLE};
use windows::Win32::System::Pipes::WaitNamedPipeW;
use windows::core::HSTRING;

use qingjian_platform::pipe::{CLIENT_ACCESS, CLIENT_QOS, UserSid, pipe_name, server_is_trusted};

use crate::com::service::launch::server_exe;

/// 连好的命名管道。对端关闭时读到 EOF；`flush` 是空操作（管道上 `FlushFileBuffers` 会阻塞到对端读完）。
pub type PipeStream = File;

/// 实例都被占着时等一个可用实例的超时（毫秒）与重试次数。要短：这里在应用 UI 线程上，等久了 TSF 看门狗会切走输入法。
const BUSY_WAIT_MS: u32 = 300;
const BUSY_RETRIES: u32 = 1;

/// 连本用户、本登录会话的 Server，并确认对面就是 DLL 同目录的 `qingjian-server.exe`。
pub fn connect_default() -> io::Result<PipeStream> {
    let me = UserSid::current()?;
    let expected = server_exe()
        .ok_or_else(|| io::Error::other("cannot locate qingjian-server.exe next to the DLL"))?;
    let stream = connect(&pipe_name(&me)?)?;
    if !server_is_trusted(HANDLE(stream.as_raw_handle()), &expected, &me) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "the pipe server is not our qingjian-server",
        ));
    }
    Ok(stream)
}

/// 打开管道。只要读 + 写数据的权限，模拟级别只给识别级（见 [`CLIENT_ACCESS`]、[`CLIENT_QOS`]）。
pub fn connect(name: &str) -> io::Result<PipeStream> {
    let mut attempts = 0;
    loop {
        match OpenOptions::new()
            .access_mode(CLIENT_ACCESS)
            .security_qos_flags(CLIENT_QOS)
            .share_mode(0)
            .open(name)
        {
            Err(error)
                if error.raw_os_error() == Some(ERROR_PIPE_BUSY.0 as i32)
                    && attempts < BUSY_RETRIES =>
            {
                attempts += 1;
                let _ = unsafe { WaitNamedPipeW(&HSTRING::from(name), BUSY_WAIT_MS) };
            }
            result => return result,
        }
    }
}
