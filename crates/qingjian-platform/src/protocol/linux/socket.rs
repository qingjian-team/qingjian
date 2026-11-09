//! Server 监听、前端连接的那个 Unix socket 的位置。
//!
//! 放在协议这一层是因为**它和线上格式一样是两端的约定**：Server 按它 bind，Fcitx5 插件
//! （`apps/linux/fcitx5/src/ipc/connection.cpp`）与 IBus 前端按它 connect，三处算得不一样就是连不上。

use std::path::PathBuf;

/// 取得 Linux socket 路径；没有运行时目录时使用按用户隔离的私有临时目录。
///
/// `QINGJIAN_SOCKET` 覆盖一切（测试与多实例用），其次 `$XDG_RUNTIME_DIR/qingjian.sock`。
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
