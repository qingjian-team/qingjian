//! Unix socket 服务：连接内会话编号隔离、握手校验、断线回收和私有权限。
use crate::dispatch::Router;
use qingjian_platform::protocol::{ClientMessage, ServerMessage};
use std::io;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
mod connection;
mod session;
use connection::serve_connection;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::thread;
use std::time::Duration;

/// 主线程请求队列；容量有限，损坏客户端不能无限占用内存。
type Request = (serde_json::Value, mpsc::Sender<Option<serde_json::Value>>);
static STOP: AtomicBool = AtomicBool::new(false);
const MAX_CLIENT_CONNECTIONS: usize = 64;
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
                    if CONNECTIONS.fetch_add(1, Ordering::Relaxed) >= MAX_CLIENT_CONNECTIONS {
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

#[cfg(test)]
mod tests;
