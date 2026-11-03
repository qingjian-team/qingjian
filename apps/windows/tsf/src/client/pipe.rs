//! `cfg(windows)`：打开本用户的命名管道，认过对端再把双工流交给 [`EngineClient`](super::EngineClient)。

use std::fs::{File, OpenOptions};
use std::io;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;

use windows::Win32::Foundation::ERROR_PIPE_BUSY;
use windows::Win32::System::Pipes::WaitNamedPipeW;
use windows::core::HSTRING;

use qingjian_platform::pipe::{PeerCheck, current_name, verify_server};

use crate::com::log::log;
use crate::com::service::launch::server_exe;

/// 连好的命名管道。对端关闭时读到 EOF；`flush` 是空操作（管道上 `FlushFileBuffers` 会阻塞到对端读完）。
pub type PipeStream = File;

/// 实例都被占着时等一个可用实例的超时（毫秒）与重试次数。要短：这里在应用 UI 线程上，等久了 TSF 看门狗会切走输入法。
const BUSY_WAIT_MS: u32 = 300;
const BUSY_RETRIES: u32 = 1;

/// 连本用户的 Server。管道名按用户 SID 派生（见 [`qingjian_platform::pipe`]），别人的 Server 名字不一样。
pub fn connect_default() -> io::Result<PipeStream> {
    connect(&current_name()?)
}

/// 连上 `name` 并认对端。认不出来就按连不上处理：宁可用不了输入法，也不能把宿主的按键与光标前后文
/// 交给占了这条管道的冒名者——管道名在全局命名空间里，谁都能抢在 Server 前面建一个同名的。
pub fn connect(name: &str) -> io::Result<PipeStream> {
    let stream = open(name)?;
    let Some(expected) = server_exe() else {
        // 连自己在哪都不知道就没法认对端；这种进程本来也拉不起 Server，放行字母更合适
        return Err(io::Error::other(
            "取不到本 DLL 的路径，无法确认管道对端是不是 qingjian-server",
        ));
    };
    match verify_server(stream.as_raw_handle(), &expected) {
        Ok(PeerCheck::Image) => Ok(stream),
        // AppContainer 里的 DLL 打不开桌面进程，只能认到「管道所有者是本用户」这一层
        Ok(PeerCheck::Owner) => {
            log("管道对端只验到所有者（本用户），读不到它的进程信息");
            Ok(stream)
        }
        Err(error) => Err(error),
    }
}

/// 打开管道；实例都被占着时等一会儿再试。
fn open(name: &str) -> io::Result<PipeStream> {
    let mut attempts = 0;
    loop {
        match OpenOptions::new()
            .read(true)
            .write(true)
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
