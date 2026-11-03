//! 命名管道传输：在 `\\.\pipe\qingjian-<用户摘要>` 上服务 DLL 客户端，字节模式，帧由协议 codec 切。
//!
//! 每个应用进程各开一条连接且失焦后连接仍在，所以不能串行服务（新聚焦的应用连不上会阻塞 UI 线程、
//! 触发 TSF 看门狗）：后台接受循环每来一个客户端就新建实例、起一条线程；[`Router`] 不跨线程，
//! 留在调用线程跑工人循环，各连接经通道把消息转给它串行处理。
//!
//! 管道名在全局命名空间里，谁都能抢建同名实例，所以每条连接在读进第一帧之后先过 [`verify_client`]：
//! 不是本用户的直接断开，那一帧也不处理、不回话。

use std::fs::File;
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle};
use std::sync::atomic::{AtomicUsize, Ordering};
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

use qingjian_platform::pipe::verify_client;
use qingjian_platform::protocol::{ClientMessage, ServerMessage};

use super::{Handled, Work};
use crate::dispatch::Router;

const BUFFER_SIZE: u32 = 64 * 1024;

/// 同时在线的客户端连接上限。正常一个应用一条，几十条就算多了；上限只用来兜住「有人在刷连接」——
/// 每条连接占一条线程加一个会话表条目，不封顶的话谁都能把 Server 的线程与内存用干净。
///
/// 挡的是资源耗尽，不是拒绝服务本身：把名额占满照样能让新应用连不上，那要鉴权之后按用户限速才能解决。
const MAX_CONNECTIONS: usize = 256;

/// 当前活着的连接数，连接线程退出时由 [`ConnectionSlot`] 还回去。
static LIVE_CONNECTIONS: AtomicUsize = AtomicUsize::new(0);

/// 一条连接的占位。拿到就说明没超上限，随连接线程一起结束、自动把名额还回来。
struct ConnectionSlot;

impl ConnectionSlot {
    /// 先加再看：并发下可能多放进一两条，但绝不会漏减（漏减会让名额永久少一个，比多放更糟）。
    fn acquire() -> Option<Self> {
        if LIVE_CONNECTIONS.fetch_add(1, Ordering::Relaxed) >= MAX_CONNECTIONS {
            LIVE_CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
            return None;
        }
        Some(Self)
    }
}

impl Drop for ConnectionSlot {
    fn drop(&mut self) {
        LIVE_CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
    }
}

/// 在命名管道上服务多个客户端。当前线程独占 [`Router`] 跑工人循环，正常不返回。
/// `work` 通道由调用方建（UI 线程也往里投状态条事件），这里拿一份发送端给各连接。
///
/// `name` 由调用方按当前用户派生（[`qingjian_platform::pipe::current_name`]），一个用户一条，
/// 快速切换用户时各人的 Server 不会互抢。第一个实例带 `FILE_FLAG_FIRST_PIPE_INSTANCE`：名字已被占
/// （同一个用户的另一个 Server，或抢注者）就建不出来，直接报错退出——两个 Server 会各画一条状态条、
/// 各持一份状态。
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
            return Err(io::Error::other(format!(
                "命名管道 {name} 已被占用（另一个 qingjian-server 或抢占者），本进程退出"
            )));
        }
        Err(error) => return Err(error),
    };
    let waker = sender.clone();
    router.set_rescoring_wake(std::sync::Arc::new(move || {
        let _ = waker.send(Work::Wake);
    }));
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
            Ok(Work::Client(conn, message, reply)) => {
                let _ = reply.send(router.handle_from(conn, message));
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
///
/// 线程数封在 [`MAX_CONNECTIONS`]：名额满了就把刚连上的这条直接关掉（对端读到断管），
/// 不建实例、不进 [`Router`]。
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
        match ConnectionSlot::acquire() {
            Some(slot) => {
                let sender = sender.clone();
                thread::spawn(move || {
                    // 名额跟着线程走：线程一结束（对端断开）就还回去
                    let _slot = slot;
                    serve_connection(stream, sender);
                });
            }
            None => {
                tracing::warn!(limit = MAX_CONNECTIONS, "连接数已达上限，断开新连接");
                // 句柄随 `stream` 一起关掉，对端连不上
                drop(stream);
            }
        }
        instance = create_instance(name, descriptor, false);
    }
}

/// 转换失败返回 null，退回默认 DACL。描述符建一次、随进程存活。
fn pipe_security_descriptor() -> PSECURITY_DESCRIPTOR {
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    let converted = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            &HSTRING::from(qingjian_platform::pipe::SDDL),
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
///
/// 断开后把本连接上开过的会话照单 `CloseSession`：DLL 来不及发（应用被关 / 崩了），
/// 会话留在 Router 里会让聚焦会话的组句与候选窗口一直挂着，直到别的应用来一个键把它挤掉。
fn serve_connection(mut stream: File, sender: Sender<Work>) {
    let (reply_sender, reply_receiver) = mpsc::channel::<Option<ServerMessage>>();
    let mut opened = Vec::new();
    // 这条连接的身份：会话归属按它判（见 `Router::admits`），别处猜得到会话号也动不了。
    let conn = super::next_connection();
    // 验身份卡在第一条消息上：`ImpersonateNamedPipeClient` 要等管道上有数据被读过才能用，只能
    // 「收下第一帧 → 验 → 再处理」。原始句柄是裸指针，拷一份带进闭包，不借用 `stream`。
    let handle = stream.as_raw_handle();
    let mut verified = false;
    let outcome = super::pump(
        &mut stream,
        conn,
        |conn, message| {
            if !verified {
                verified = true;
                if let Err(error) = verify_client(handle) {
                    // 不回话：对面不是本用户，连它发来的第一帧都不处理
                    tracing::warn!(%error, "命名管道客户端的用户与本进程不同，断开");
                    return Handled::Stop;
                }
            }
            if sender
                .send(Work::Client(conn, message, reply_sender.clone()))
                .is_err()
            {
                return Handled::Stop;
            }
            match reply_receiver.recv() {
                Ok(reply) => Handled::Reply(reply.map(Box::new)),
                Err(_) => Handled::Stop,
            }
        },
        &mut opened,
    );
    if let Err(error) = outcome {
        tracing::warn!(%error, "客户端会话读出错");
    }
    for session in opened {
        // 工人线程已经不在了就没人收，丢掉即可
        let _ = sender.send(Work::Client(
            conn,
            ClientMessage::CloseSession { session },
            reply_sender.clone(),
        ));
    }
    let _ = unsafe { DisconnectNamedPipe(HANDLE(stream.as_raw_handle())) };
    tracing::debug!("客户端断开");
}
