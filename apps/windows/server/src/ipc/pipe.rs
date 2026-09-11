//! Windows 命名管道传输：在 `\\.\pipe\qingjian` 上服务 DLL 客户端。字节模式，帧由协议层 codec 切。
//!
//! 每个应用进程里的 DLL 各开一条连接，而且失焦后连接仍在（TSF 不保证及时 `Deactivate`），所以不能串行
//! 一次只服务一个（新聚焦的应用连不上会阻塞 UI 线程，触发 TSF 看门狗切走输入法）。做法：后台一条接受
//! 循环，每来一个客户端就新建管道实例并起一条线程服务它；[`Router`] 不跨线程，留在调用线程跑工人循环，
//! 各连接把消息经通道转给它串行处理。

use std::io::{self, Read, Write};
use std::ptr;
use std::sync::mpsc::{self, Sender};
use std::thread;

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_BROKEN_PIPE, ERROR_PIPE_CONNECTED, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    FlushFileBuffers, PIPE_ACCESS_DUPLEX, ReadFile, WriteFile,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE,
    PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};

use qingjian_platform::protocol::{ClientMessage, ServerMessage, read_message, write_message};

use crate::dispatch::Router;

pub use qingjian_platform::protocol::DEFAULT_PIPE_NAME;

/// 一条连接转给工人线程的一次请求：消息 + 回结果的通道。
type Request = (ClientMessage, Sender<Option<ServerMessage>>);

/// 每个方向的管道缓冲区大小。
const BUFFER_SIZE: u32 = 64 * 1024;

/// 一个命名管道实例的句柄，析构时关闭。
struct PipeStream(HANDLE);

// SAFETY: 句柄由本对象独占，移动到服务线程后只在那条线程上用。
unsafe impl Send for PipeStream {}

impl Drop for PipeStream {
    fn drop(&mut self) {
        // SAFETY: 句柄由 CreateNamedPipeW 得来、本对象独占，只在这里关一次。
        unsafe { CloseHandle(self.0) };
    }
}

impl Read for PipeStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let want = buf.len().min(u32::MAX as usize) as u32;
        let mut read = 0u32;
        // SAFETY: buf 可写 want 字节，read 是有效指针，无重叠 IO。
        let ok = unsafe { ReadFile(self.0, buf.as_mut_ptr(), want, &mut read, ptr::null_mut()) };
        if ok == 0 {
            let error = io::Error::last_os_error();
            // 对端关闭当作 EOF，serve 据此干净结束。
            return if error.raw_os_error() == Some(ERROR_BROKEN_PIPE as i32) {
                Ok(0)
            } else {
                Err(error)
            };
        }
        Ok(read as usize)
    }
}

impl Write for PipeStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let want = buf.len().min(u32::MAX as usize) as u32;
        let mut written = 0u32;
        // SAFETY: buf 可读 want 字节，written 是有效指针，无重叠 IO。
        let ok = unsafe { WriteFile(self.0, buf.as_ptr(), want, &mut written, ptr::null_mut()) };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(written as usize)
    }

    fn flush(&mut self) -> io::Result<()> {
        // SAFETY: 句柄有效。
        if unsafe { FlushFileBuffers(self.0) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

/// 建一个新的管道实例并等一个客户端连上。
fn accept(name_wide: &[u16]) -> io::Result<PipeStream> {
    // SAFETY: name_wide 以 0 结尾；安全属性传 null 用默认 DACL。
    let handle = unsafe {
        CreateNamedPipeW(
            name_wide.as_ptr(),
            PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
            PIPE_UNLIMITED_INSTANCES,
            BUFFER_SIZE,
            BUFFER_SIZE,
            0,
            ptr::null(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let stream = PipeStream(handle);
    // SAFETY: 句柄有效，阻塞式（无 overlapped）。
    if unsafe { ConnectNamedPipe(handle, ptr::null_mut()) } == 0 {
        let error = io::Error::last_os_error();
        // 客户端在建实例与 ConnectNamedPipe 之间就连上了：也算成功。
        if error.raw_os_error() != Some(ERROR_PIPE_CONNECTED as i32) {
            return Err(error);
        }
    }
    Ok(stream)
}

/// 在命名管道上服务多个客户端。后台线程跑接受循环，当前线程独占 [`Router`] 跑工人循环。
/// 只在接受循环退出且所有连接都断开后才返回，正常不会发生。
pub fn serve_pipe(name: &str, router: &mut Router) -> io::Result<()> {
    let name_wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let (sender, receiver) = mpsc::channel::<Request>();
    thread::spawn(move || accept_loop(name_wide, sender));

    tracing::info!(pipe = name, "命名管道监听中");
    while let Ok((message, reply)) = receiver.recv() {
        let _ = reply.send(router.handle(message));
    }
    Ok(())
}

/// 每建一个实例、等一个客户端连上，就起一条线程服务它。建实例出错才停。
fn accept_loop(name_wide: Vec<u16>, sender: Sender<Request>) {
    loop {
        match accept(&name_wide) {
            Ok(stream) => {
                let sender = sender.clone();
                thread::spawn(move || serve_connection(stream, sender));
            }
            Err(error) => {
                tracing::error!(%error, "建管道实例失败，停止接受");
                break;
            }
        }
    }
}

/// 服务一条连接：读消息 → 转给工人线程 → 把回复写回，直到对端在帧边界关闭或出错。
fn serve_connection(mut stream: PipeStream, sender: Sender<Request>) {
    let (reply_sender, reply_receiver) = mpsc::channel::<Option<ServerMessage>>();
    loop {
        match read_message::<_, ClientMessage>(&mut stream) {
            Ok(Some(message)) => {
                if sender.send((message, reply_sender.clone())).is_err() {
                    break;
                }
                match reply_receiver.recv() {
                    Ok(Some(response)) => {
                        if write_message(&mut stream, &response).is_err() {
                            break;
                        }
                    }
                    Ok(None) => {}
                    Err(_) => break,
                }
            }
            Ok(None) => break,
            Err(error) => {
                tracing::warn!(%error, "客户端会话读出错");
                break;
            }
        }
    }
    // SAFETY: 句柄有效；断开这个客户端，句柄随 stream 析构关闭。
    unsafe { DisconnectNamedPipe(stream.0) };
    tracing::debug!("客户端断开");
}
