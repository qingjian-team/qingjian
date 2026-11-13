//! 命名管道传输：在按用户与登录会话命名的管道上服务 DLL 客户端（名字与身份校验见 [`qingjian_platform::pipe`]），
//! 字节模式，帧由协议 codec 切。
//!
//! 每个应用进程各开一条连接且失焦后连接仍在，所以不能串行服务（新聚焦的应用连不上会阻塞 UI 线程、
//! 触发 TSF 看门狗）：后台接受循环每来一个客户端就新建实例、起一条线程；[`Router`] 不跨线程，
//! 留在调用线程跑工人循环，各连接经通道把消息转给它串行处理。

use std::fs::File;
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle};
use std::sync::Arc;
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
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, GetNamedPipeClientProcessId,
    PIPE_READMODE_BYTE, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};
use windows::core::{HRESULT, HSTRING};

use qingjian_platform::pipe::{LEGACY_PIPE_SDDL, UserSid, client_is_same_user, pipe_sddl};
use qingjian_platform::protocol::{ClientMessage, ServerMessage};

use super::limit::ConnectionLimit;
use super::listener::Listener;
use super::sessions::ConnectionSessions;
use super::{Handled, Work};
use crate::dispatch::Router;

const BUFFER_SIZE: u32 = 64 * 1024;

/// 在命名管道上服务多个客户端。当前线程独占 [`Router`] 跑工人循环，正常不返回。
/// `work` 通道由调用方建（UI 线程也往里投状态条事件），这里拿一份发送端给各连接。
/// 第一个实例带 `FILE_FLAG_FIRST_PIPE_INSTANCE`：已有一个 Server 在跑就建不出来，直接报错退出（两个 Server
/// 会各画一条状态条、各持一份状态）。
///
/// `legacy` 是老 DLL 连的旧管道名：升级后没重启的应用里还是老 DLL，同时在旧名字上服务；
/// 建不出来（被占了）只记一笔，不影响新管道。
pub fn serve_pipe(
    name: &str,
    legacy: Option<&str>,
    router: &mut Router,
    sender: Sender<Work>,
    receiver: Receiver<Work>,
) -> io::Result<()> {
    let me = UserSid::current()?;
    let sddl = pipe_sddl(&me)?;
    let first = match create_instance(&HSTRING::from(name), security_descriptor(&sddl), true) {
        Ok(first) => first,
        Err(error) if error.raw_os_error() == Some(ERROR_ACCESS_DENIED.0 as i32) => {
            return Err(io::Error::other(
                "named pipe already exists (another qingjian-server, or another process holding the name), exiting",
            ));
        }
        Err(error) => return Err(error),
    };
    let limit = Arc::new(ConnectionLimit::default());
    if let Some(legacy) = legacy {
        let legacy_first = create_instance(
            &HSTRING::from(legacy),
            security_descriptor(LEGACY_PIPE_SDDL),
            true,
        );
        match legacy_first {
            Ok(first) => {
                let listener = Listener {
                    name: legacy.to_owned(),
                    sddl: LEGACY_PIPE_SDDL.to_owned(),
                    me: me.clone(),
                    limit: Arc::clone(&limit),
                };
                let sender = sender.clone();
                thread::spawn(move || accept_loop(listener, first, sender));
                tracing::info!(pipe = legacy, "Legacy named pipe listening");
            }
            Err(error) => tracing::warn!(
                %error,
                pipe = legacy,
                "Legacy named pipe unavailable; apps still running an old DLL reconnect only after a restart"
            ),
        }
    }
    let waker = sender.clone();
    router.set_rescoring_wake(Arc::new(move || {
        let _ = waker.send(Work::Wake);
    }));
    let listener = Listener {
        name: name.to_owned(),
        sddl,
        me,
        limit,
    };
    thread::spawn(move || accept_loop(listener, first, sender));
    tracing::info!(pipe = name, "Named pipe listening");
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
            Ok(Work::Click(mouse)) => router.handle_candidate_mouse(mouse),
            Ok(Work::Panel(hit)) => router.handle_panel_mouse(hit),
            Ok(Work::Wake) => router.tick(),
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => {
                tracing::warn!(
                    reason = "all_work_senders_disconnected",
                    "Pipe worker loop stopped"
                );
                break;
            }
        }
        // 处理完消息节拍可能变短了（按键起了防抖）：到点时间只提前不推后
        due = due.min(Instant::now() + router.next_tick());
    }
    Ok(())
}

/// 先在建好的第一个实例上等客户端，之后每建一个实例、等一个客户端连上，就起一条线程服务它。建实例出错才停。
/// 同一进程的连接数到了上限，接上就断开。
fn accept_loop(listener: Listener, first: File, sender: Sender<Work>) {
    let pipe = HSTRING::from(listener.name.as_str());
    let descriptor = security_descriptor(&listener.sddl);
    let mut instance = Ok(first);
    loop {
        let stream = match instance.and_then(wait_client) {
            Ok(stream) => stream,
            Err(error) => {
                tracing::error!(%error, pipe = listener.name, "Creating a pipe instance failed, no longer accepting");
                break;
            }
        };
        let pid = client_process_id(&stream);
        match listener.limit.admit(pid) {
            Some(slot) => {
                let sender = sender.clone();
                let me = listener.me.clone();
                thread::spawn(move || {
                    serve_connection(stream, sender, &me);
                    drop(slot);
                });
            }
            None => {
                tracing::warn!(
                    pid,
                    "Client process has too many pipe connections, refusing"
                );
                let _ = unsafe { DisconnectNamedPipe(HANDLE(stream.as_raw_handle())) };
            }
        }
        instance = create_instance(&pipe, descriptor, false);
    }
}

/// 管道对面的进程号；查不到记成 0（这类连接共用一份名额）。
fn client_process_id(stream: &File) -> u32 {
    let mut pid = 0u32;
    let _ = unsafe { GetNamedPipeClientProcessId(HANDLE(stream.as_raw_handle()), &mut pid) };
    pid
}

/// 转换失败返回 null，退回默认 DACL。描述符建一次、随进程存活。
fn security_descriptor(sddl: &str) -> PSECURITY_DESCRIPTOR {
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    let converted = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            &HSTRING::from(sddl),
            SDDL_REVISION_1,
            &mut descriptor,
            None,
        )
    };
    if let Err(error) = converted {
        tracing::warn!(
            %error,
            "Building the pipe security descriptor failed, falling back to the default DACL (AppContainer apps cannot connect)"
        );
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
/// 收到第一帧时先认人：客户端不是 `me` 这个用户就断开，那一帧也不处理。
///
/// 断开后把本连接上开过的会话照单 `CloseSession`：DLL 来不及发（应用被关 / 崩了），
/// 会话留在 Router 里会让聚焦会话的组句与候选窗口一直挂着，直到别的应用来一个键把它挤掉。
fn serve_connection(mut stream: File, sender: Sender<Work>, me: &UserSid) {
    let (reply_sender, reply_receiver) = mpsc::channel::<Option<ServerMessage>>();
    let mut sessions = ConnectionSessions::default();
    let pipe = HANDLE(stream.as_raw_handle());
    let mut verified = false;
    let outcome = super::pump(
        &mut stream,
        |message| {
            if !verified {
                if !client_is_same_user(pipe, me) {
                    tracing::warn!("Pipe client is not the server's user, disconnecting");
                    return Handled::Stop;
                }
                verified = true;
            }
            if sender
                .send(Work::Client(message, reply_sender.clone()))
                .is_err()
            {
                return Handled::Stop;
            }
            match reply_receiver.recv() {
                Ok(reply) => Handled::Reply(reply.map(Box::new)),
                Err(_) => Handled::Stop,
            }
        },
        &mut sessions,
    );
    if let Err(error) = outcome {
        tracing::warn!(%error, "Reading from the pipe client failed");
    }
    for session in sessions.drain() {
        // 工人线程已经不在了就没人收，丢掉即可
        let _ = sender.send(Work::Client(
            ClientMessage::CloseSession { session },
            reply_sender.clone(),
        ));
    }
    let _ = unsafe { DisconnectNamedPipe(HANDLE(stream.as_raw_handle())) };
    tracing::debug!("Pipe client disconnected");
}
