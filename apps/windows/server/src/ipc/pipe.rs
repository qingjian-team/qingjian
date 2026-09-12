//! 命名管道传输：在 `\\.\pipe\qingjian` 上服务 DLL 客户端，字节模式，帧由协议 codec 切。
//!
//! 每个应用进程各开一条连接且失焦后连接仍在，所以不能串行服务（新聚焦的应用连不上会阻塞 UI 线程、
//! 触发 TSF 看门狗）：后台接受循环每来一个客户端就新建实例、起一条线程；[`Router`] 不跨线程，
//! 留在调用线程跑工人循环，各连接经通道把消息转给它串行处理。

use std::fs::File;
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::Instant;

use windows::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_PIPE_CONNECTED, HANDLE};
use windows::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
use windows::Win32::Storage::FileSystem::{FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE,
    PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};
use windows::core::{HRESULT, HSTRING};

use qingjian_platform::protocol::{ClientMessage, ServerMessage, read_message, write_message};

use super::Work;
use crate::dispatch::Router;

pub use qingjian_platform::protocol::DEFAULT_PIPE_NAME;

const BUFFER_SIZE: u32 = 64 * 1024;

/// 管道的 SDDL：放行 Everyone / ALL APPLICATION PACKAGES / ALL RESTRICTED APPLICATION PACKAGES，
/// 完整性标 Low。任务栏搜索、设置这类 AppContainer 进程在默认 DACL 下连不上。
const PIPE_SDDL: &str = "D:(A;;GA;;;WD)(A;;GA;;;AC)(A;;GA;;;S-1-15-2-2)S:(ML;;NW;;;LW)";

/// 在命名管道上服务多个客户端。当前线程独占 [`Router`] 跑工人循环，正常不返回。
/// `work` 通道由调用方建（UI 线程也往里投状态条事件），这里拿一份发送端给各连接。
/// 第一个实例带 `FILE_FLAG_FIRST_PIPE_INSTANCE`：已有一个 Server 在跑就建不出来，直接报错退出（两个 Server
/// 会各画一条状态条、各持一份状态）。
pub fn serve_pipe(
    name: &str,
    router: &mut Router,
    sender: Sender<Work>,
    receiver: Receiver<Work>,
) -> io::Result<()> {
    let pipe = HSTRING::from(name);
    let first = match create_instance(&pipe, pipe_security_descriptor(), true) {
        Ok(first) => first,
        Err(error) if error.raw_os_error() == Some(ERROR_ACCESS_DENIED.0 as i32) => {
            return Err(io::Error::other(
                "已有一个 qingjian-server 在运行（命名管道被占），本进程退出",
            ));
        }
        Err(error) => return Err(error),
    };
    thread::spawn(move || accept_loop(&pipe, first, sender));
    tracing::info!(pipe = name, "命名管道监听中");
    // 按 Router 的节拍来 tick：在等本地整句模型就几十毫秒一次，否则一秒看一次配置文件。
    // 到点时间是绝对的，不随消息重新计时——前台进程里的 DLL 隔几百毫秒就问一次切模式（SyncMode），
    // 若每收一条消息就重等一秒，tick 永远到不了，热加载与模型接入都会停摆。
    let mut due = Instant::now() + router.next_tick();
    loop {
        let now = Instant::now();
        if now >= due {
            router.tick();
            due = Instant::now() + router.next_tick();
            continue;
        }
        match receiver.recv_timeout(due - now) {
            Ok(Work::Client(message, reply)) => {
                let _ = reply.send(router.handle(message));
            }
            Ok(Work::Status(event)) => router.handle_status_event(event),
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        }
        // 处理完消息节拍可能变短了（按键起了防抖）：到点时间只提前不推后
        due = due.min(Instant::now() + router.next_tick());
    }
    Ok(())
}

/// 先在建好的第一个实例上等客户端，之后每建一个实例、等一个客户端连上，就起一条线程服务它。建实例出错才停。
fn accept_loop(name: &HSTRING, first: File, sender: Sender<Work>) {
    let descriptor = pipe_security_descriptor();
    let mut instance = Ok(first);
    loop {
        let stream = match instance.and_then(wait_client) {
            Ok(stream) => stream,
            Err(error) => {
                tracing::error!(%error, "建管道实例失败，停止接受");
                break;
            }
        };
        let sender = sender.clone();
        thread::spawn(move || serve_connection(stream, sender));
        instance = create_instance(name, descriptor, false);
    }
}

/// 转换失败返回 null，退回默认 DACL。描述符建一次、随进程存活。
fn pipe_security_descriptor() -> PSECURITY_DESCRIPTOR {
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    let converted = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            &HSTRING::from(PIPE_SDDL),
            SDDL_REVISION_1,
            &mut descriptor,
            None,
        )
    };
    if let Err(error) = converted {
        tracing::warn!(%error, "构建管道安全描述符失败，退回默认 DACL（UWP 应用可能连不上）");
        return PSECURITY_DESCRIPTOR::default();
    }
    descriptor
}

/// 建一个实例。句柄交给 `File` 管：对端关闭时 std 把 `ERROR_BROKEN_PIPE` 当 EOF。
fn create_instance(
    name: &HSTRING,
    descriptor: PSECURITY_DESCRIPTOR,
    first: bool,
) -> io::Result<File> {
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: false.into(),
    };
    let attributes = (!descriptor.0.is_null()).then_some(&raw const attributes);
    let mut open_mode = PIPE_ACCESS_DUPLEX;
    if first {
        open_mode |= FILE_FLAG_FIRST_PIPE_INSTANCE;
    }
    let handle = unsafe {
        CreateNamedPipeW(
            name,
            open_mode,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
            PIPE_UNLIMITED_INSTANCES,
            BUFFER_SIZE,
            BUFFER_SIZE,
            0,
            attributes,
        )
    };
    if handle.is_invalid() {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_handle(handle.0) })
}

/// 阻塞等一个客户端连上这个实例。
fn wait_client(stream: File) -> io::Result<File> {
    let handle = HANDLE(stream.as_raw_handle());
    // 客户端在建实例与 ConnectNamedPipe 之间就连上了也算成功。
    if let Err(error) = unsafe { ConnectNamedPipe(handle, None) }
        && error.code() != HRESULT::from_win32(ERROR_PIPE_CONNECTED.0)
    {
        return Err(io::Error::other(error));
    }
    Ok(stream)
}

/// 服务一条连接：读消息 → 转给工人线程 → 写回，直到对端在帧边界关闭或出错。
fn serve_connection(mut stream: File, sender: Sender<Work>) {
    let (reply_sender, reply_receiver) = mpsc::channel::<Option<ServerMessage>>();
    loop {
        let message = match read_message::<_, ClientMessage>(&mut stream) {
            Ok(Some(message)) => message,
            Ok(None) => break,
            Err(error) => {
                tracing::warn!(%error, "客户端会话读出错");
                break;
            }
        };
        if sender
            .send(Work::Client(message, reply_sender.clone()))
            .is_err()
        {
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
    let _ = unsafe { DisconnectNamedPipe(HANDLE(stream.as_raw_handle())) };
    tracing::debug!("客户端断开");
}
