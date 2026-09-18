//! Unix socket 服务：连接内会话编号隔离、握手校验、断线回收和私有权限。
use crate::dispatch::Router;
use qingjian_platform::protocol::{
    ClientMessage, Frame, PROTOCOL_VERSION, ServerMessage, SessionId, read_message, write_message,
};
use std::collections::HashMap;
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::thread;
use std::time::Duration;

/// 主线程请求队列；容量有限，损坏客户端不能无限占用内存。
type Request = (serde_json::Value, mpsc::Sender<Option<serde_json::Value>>);
static STOP: AtomicBool = AtomicBool::new(false);
static CONNECTIONS: AtomicUsize = AtomicUsize::new(0);
/// 主线程在下一次空闲节拍退出，落盘由 Router::drop 完成。
pub fn request_shutdown() {
    STOP.store(true, Ordering::Relaxed);
}
static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

/// 取得 Linux socket 路径；没有运行时目录时使用按用户隔离的私有临时目录。
pub fn socket_path() -> PathBuf {
    std::env::var_os("QINGJIAN_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("XDG_RUNTIME_DIR")
                .filter(|p| !p.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(format!("/tmp/qingjian-{}", unsafe { libc::geteuid() }))
                })
                .join("qingjian.sock")
        })
}

/// 仅移除同用户、确认拒绝连接的陈旧 socket；拒绝符号链接和不安全的父目录。
pub fn bind_socket(path: &Path) -> io::Result<UnixListener> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| io::Error::other("socket requires an absolute parent directory"))?;
    if !path.is_absolute() {
        return Err(io::Error::other("socket path must be absolute"));
    }
    if !parent.exists() {
        std::fs::create_dir_all(parent)?;
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
    }
    let owner = unsafe { libc::geteuid() };
    let metadata = std::fs::symlink_metadata(parent)?;
    if !metadata.is_dir() || metadata.uid() != owner || metadata.mode() & 0o022 != 0 {
        return Err(io::Error::other(
            "socket directory must be owned by this user and not writable by others",
        ));
    }
    match std::fs::symlink_metadata(path) {
        Ok(meta) => {
            if !meta.file_type().is_socket() || meta.uid() != owner {
                return Err(io::Error::other("unsafe socket path"));
            }
            match UnixStream::connect(path) {
                Ok(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::AddrInUse,
                        "server already running",
                    ));
                }
                Err(error) if error.kind() == io::ErrorKind::ConnectionRefused => {
                    std::fs::remove_file(path)?
                }
                Err(error) => return Err(error),
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let listener = UnixListener::bind(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(listener)
}

pub fn serve_socket(path: impl AsRef<Path>, router: &mut Router) -> io::Result<()> {
    let listener = bind_socket(path.as_ref())?;
    let settings = router.display_settings();
    let (sender, receiver) = mpsc::sync_channel::<Request>(128);
    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    if CONNECTIONS.fetch_add(1, Ordering::Relaxed) >= 64 {
                        CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
                        continue;
                    }
                    let sender = sender.clone();
                    let settings = settings.clone();
                    thread::spawn(move || {
                        serve_connection(stream, sender, settings);
                        CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
                    });
                }
                Err(error) => {
                    tracing::error!(%error, "接受客户端连接失败");
                    break;
                }
            }
        }
    });
    while !STOP.load(Ordering::Relaxed) {
        match receiver.recv_timeout(Duration::from_secs(1)) {
            Ok((message, reply)) => {
                let _ = reply.send(router.handle_linux(message));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => router.tick(),
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
        }
    }
    std::fs::remove_file(path.as_ref())?;
    Ok(())
}

fn dispatch(sender: &SyncSender<Request>, message: ClientMessage) -> Option<ServerMessage> {
    let (reply, receiver) = mpsc::channel();
    sender
        .send((serde_json::to_value(message).ok()?, reply))
        .ok()?;
    serde_json::from_value(receiver.recv().ok().flatten()?).ok()
}

fn dispatch_json(
    sender: &SyncSender<Request>,
    message: serde_json::Value,
) -> Option<serde_json::Value> {
    let (reply, receiver) = mpsc::channel();
    sender.send((message, reply)).ok()?;
    receiver.recv().ok().flatten()
}

fn serve_connection(
    mut stream: UnixStream,
    sender: SyncSender<Request>,
    settings: serde_json::Value,
) {
    let mut credentials = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut size = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    let trusted = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut credentials as *mut libc::ucred).cast(),
            &mut size,
        ) == 0
            && credentials.uid == libc::geteuid()
    };
    if !trusted {
        return;
    }
    // 阻塞写上限：不读取答复的客户端不能无限占住线程。
    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
    let mut sessions = HashMap::<SessionId, SessionId>::new();
    let mut hello = None::<crate::protocol::DisplayIdentity>;
    while let Ok(Some(value)) = read_message::<_, serde_json::Value>(&mut stream) {
        if let Some(request) = value.get("LinuxHello") {
            if hello.is_some()
                || request.get("version").and_then(|v| v.as_u64())
                    != Some(u64::from(crate::protocol::LINUX_UI_PROTOCOL))
            {
                break;
            }
            let Some(context) = request
                .get("context")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty() && s.len() <= 64)
            else {
                break;
            };
            let Some(generation) = request.get("generation").and_then(|v| v.as_u64()) else {
                break;
            };
            hello = Some(crate::protocol::DisplayIdentity {
                generation,
                context: context.into(),
                revision: 0,
            });
            for global in sessions.values() {
                dispatch_json(
                    &sender,
                    serde_json::json!({"DisplayReporting": {"session": global, "identity": hello}}),
                );
            }
            if write_message(&mut stream, &serde_json::json!({"LinuxHello": settings})).is_err() {
                break;
            }
            continue;
        }
        if hello.is_none() && value.get("OpenSession").is_none() {
            break;
        }
        if let Some(body) = value.get("LinuxEvent") {
            let Ok(mut request) =
                serde_json::from_value::<crate::protocol::LinuxRequest>(body.clone())
            else {
                break;
            };
            let local = request.session;
            let Some(global) = sessions.get(&local) else {
                break;
            };
            request.session = *global;
            let Some(response) = dispatch_json(&sender, serde_json::json!({"LinuxEvent": request}))
            else {
                break;
            };
            if write_message(&mut stream, &localize_json(response, local)).is_err() {
                break;
            }
            continue;
        }
        if let Some(request) = value.get("DisplayAcknowledged") {
            let Ok(mut ack) =
                serde_json::from_value::<crate::protocol::DisplayAcknowledged>(request.clone())
            else {
                break;
            };
            let Some(identity) = &hello else {
                break;
            };
            let Some(global) = sessions.get(&ack.session) else {
                break;
            };
            if ack.identity.generation != identity.generation
                || ack.identity.context != identity.context
                || ack.senses.len() > 128
            {
                break;
            }
            ack.session = *global;
            dispatch_json(&sender, serde_json::json!({"DisplayAcknowledged": ack}));
            continue;
        }
        let Ok(message) = serde_json::from_value::<ClientMessage>(value) else {
            break;
        };
        let response = match message {
            ClientMessage::OpenSession {
                session,
                app,
                protocol,
            } => {
                if protocol != PROTOCOL_VERSION || sessions.len() >= 64 {
                    break;
                }
                if let Some(old) = sessions.remove(&session) {
                    dispatch(&sender, ClientMessage::CloseSession { session: old });
                }
                let global = SessionId(NEXT_SESSION.fetch_add(1, Ordering::Relaxed));
                sessions.insert(session, global);
                dispatch(
                    &sender,
                    ClientMessage::OpenSession {
                        session: global,
                        app,
                        protocol,
                    },
                );
                if let Some(identity) = &hello {
                    dispatch_json(
                        &sender,
                        serde_json::json!({"DisplayReporting": {"session": global, "identity": identity}}),
                    );
                }
                // Linux 的 OpenSession 回含 linux_ui 的 Update，再协商 LinuxHello；
                // Windows v6 的 SessionOpened 属于 DLL 通路，不向此连接额外发送。
                Some(
                    serde_json::json!({"Update": {"session": session, "frame": Frame::default(), "linux_ui": settings}}),
                )
            }
            ClientMessage::Key { session, event } => {
                let Some(global) = sessions.get(&session) else {
                    break;
                };
                dispatch_json(
                    &sender,
                    serde_json::json!({"Key": {"session": global, "event": event}}),
                )
                .map(|reply| localize_json(reply, session))
            }
            ClientMessage::Poll { session } => {
                let Some(global) = sessions.get(&session) else {
                    break;
                };
                dispatch_json(&sender, serde_json::json!({"Poll": {"session": global}}))
                    .map(|reply| localize_json(reply, session))
            }
            ClientMessage::Commit { session } => {
                let Some(global) = sessions.get(&session) else {
                    break;
                };
                dispatch_json(&sender, serde_json::json!({"Commit": {"session": global}}))
                    .map(|reply| localize_json(reply, session))
            }
            ClientMessage::Privacy { session, private } => {
                let Some(global) = sessions.get(&session) else {
                    break;
                };
                dispatch(
                    &sender,
                    ClientMessage::Privacy {
                        session: *global,
                        private,
                    },
                );
                None
            }
            ClientMessage::CloseSession { session } => {
                let Some(global) = sessions.remove(&session) else {
                    break;
                };
                dispatch(&sender, ClientMessage::CloseSession { session: global });
                None
            }
            _ => break,
        };
        if let Some(response) = response
            && write_message(&mut stream, &response).is_err()
        {
            break;
        }
    }
    for (_, session) in sessions {
        dispatch(&sender, ClientMessage::CloseSession { session });
    }
}

fn localize_json(mut response: serde_json::Value, local: SessionId) -> serde_json::Value {
    if let Some(body) = response.as_object_mut().and_then(|o| o.values_mut().next()) {
        body["session"] = serde_json::json!(local);
    }
    response
}
