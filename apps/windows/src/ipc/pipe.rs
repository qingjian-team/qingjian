//! Windows 命名管道传输：Server 在 `\\.\pipe\qingjian` 上服务 DLL 客户端。字节模式，帧由协议层
//! codec 自己切。
//!
//! **多客户端并发**：每个应用进程里的 TSF DLL 各开一条连接，而且失焦后连接仍在（TSF 不保证及时
//! `Deactivate`）。所以不能串行一次只服务一个——那样新聚焦的应用连不上会阻塞，DLL 在 UI 线程上等，
//! 触发 TSF 看门狗把输入法切回去。做法：后台一个接受循环，每来一个客户端就新建一个管道实例并起一条
//! 线程服务它；[`Router`]（含 Engine）不跨线程，留在调用线程上跑一个工人循环，所有连接把消息经通道
//! 转给它、串行处理（同一时刻只有一个应用在敲键，锁自然不争）。只有消息与管道句柄跨线程。
//!
//! 只在 Windows 编译。DLL 侧用 `CreateFileW` 打开同名管道即可对接。

use std::io::{self, Read, Write};
use std::ptr;
use std::sync::mpsc::{self, Sender};
use std::thread;

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_BROKEN_PIPE, ERROR_PIPE_CONNECTED, GetLastError, HANDLE,
    INVALID_HANDLE_VALUE,
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

/// 一条连接转给工人线程的一次请求：消息 + 回结果的通道。
type Request = (ClientMessage, Sender<Option<ServerMessage>>);

/// 缺省管道名（Server 与 DLL 共用的字面量，定义在协议层）。这里 re-export 方便 `main` 用。
pub use qingjian_platform::protocol::DEFAULT_PIPE_NAME;

/// 每个方向的管道缓冲区大小。
const BUFFER_SIZE: u32 = 64 * 1024;

/// 一个命名管道句柄，析构时关闭。impl Read + Write，可直接交给 [`crate::ipc::serve`]。
struct PipeStream(HANDLE);

// SAFETY: 句柄由本对象独占（Drop 里只关一次），移动到某一条服务线程后只在那条线程上用，不共享。
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
            // SAFETY: 紧接失败调用取错误码。
            let code = unsafe { GetLastError() };
            // 对端关闭：当作 EOF（serve 据此干净结束）。
            if code == ERROR_BROKEN_PIPE {
                return Ok(0);
            }
            return Err(io::Error::from_raw_os_error(code as i32));
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
            // SAFETY: 紧接失败调用取错误码。
            let code = unsafe { GetLastError() };
            return Err(io::Error::from_raw_os_error(code as i32));
        }
        Ok(written as usize)
    }

    fn flush(&mut self) -> io::Result<()> {
        // SAFETY: 句柄有效。
        let ok = unsafe { FlushFileBuffers(self.0) };
        if ok == 0 {
            // SAFETY: 紧接失败调用取错误码。
            let code = unsafe { GetLastError() };
            return Err(io::Error::from_raw_os_error(code as i32));
        }
        Ok(())
    }
}

/// 建一个新的管道实例并等一个客户端连上，返回连好的流。
fn accept(name_wide: &[u16]) -> io::Result<PipeStream> {
    // SAFETY: name_wide 是以 0 结尾的宽字符串，安全属性传 null（用默认 DACL）。
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
    let connected = unsafe { ConnectNamedPipe(handle, ptr::null_mut()) };
    if connected == 0 {
        // SAFETY: 紧接失败调用取错误码。
        let code = unsafe { GetLastError() };
        // 客户端在 CreateNamedPipeW 与 ConnectNamedPipe 之间就连上了：也算成功。
        if code != ERROR_PIPE_CONNECTED {
            return Err(io::Error::from_raw_os_error(code as i32));
        }
    }
    Ok(stream)
}

/// 在命名管道上服务多个客户端。后台线程跑接受循环（每来一个客户端新建实例 + 起线程服务），
/// 当前线程独占 [`Router`] 跑工人循环处理所有连接转来的消息。永不返回（工人循环只在接受循环退出、
/// 所有发送端都掉线时才结束，正常不会发生）。
pub fn serve_pipe(name: &str, router: &mut Router) -> io::Result<()> {
    let name_wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let (sender, receiver) = mpsc::channel::<Request>();

    // 接受循环单独一条线程：只拿 sender 与管道句柄，不碰 Router。
    thread::spawn(move || accept_loop(name_wide, sender));

    tracing::info!(pipe = name, "命名管道监听中（多客户端）");
    // 工人循环：Router 留在本线程，串行处理所有连接来的消息。
    while let Ok((message, reply)) = receiver.recv() {
        let _ = reply.send(router.handle(message));
    }
    Ok(())
}

/// 接受循环：每建一个管道实例、等一个客户端连上，就起一条线程服务它，随即回来建下一个实例，
/// 于是多个客户端各占一个实例、互不阻塞。建实例本身出错才停。
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

/// 服务一条已连上的连接：循环读消息 → 转给工人线程 → 把回复写回，直到对端在帧边界关闭或出错。
fn serve_connection(mut stream: PipeStream, sender: Sender<Request>) {
    let (reply_sender, reply_receiver) = mpsc::channel::<Option<ServerMessage>>();
    loop {
        match read_message::<_, ClientMessage>(&mut stream) {
            Ok(Some(message)) => {
                if sender.send((message, reply_sender.clone())).is_err() {
                    break; // 工人线程没了
                }
                match reply_receiver.recv() {
                    Ok(Some(response)) => {
                        if write_message(&mut stream, &response).is_err() {
                            break;
                        }
                    }
                    Ok(None) => {} // 这条消息不用回话
                    Err(_) => break,
                }
            }
            Ok(None) => break, // 干净 EOF
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
