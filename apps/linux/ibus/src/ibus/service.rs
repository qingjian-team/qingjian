//! 连上 IBus 总线并把工厂挂上去。
//!
//! IBus 不走会话总线，自己起一条。地址三处找：ibus-daemon 拉起我们时设的 `IBUS_ADDRESS`
//! 环境变量（正常情况走这条）、`ibus address` 会读的那个地址文件、以及用户手动指定的。

use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

use zbus::connection::Builder;

use super::component::COMPONENT_NAME;
use super::factory::IBusFactory;
use crate::client::Shared;

/// 工厂对象挂在这个路径，IBus 写死的。
const FACTORY_PATH: &str = "/org/freedesktop/IBus/Factory";

/// 连上总线、挂好工厂、占住组件名。返回的连接活着服务就活着。
pub async fn serve(client: Shared) -> Result<zbus::Connection, zbus::Error> {
    let address = bus_address().ok_or_else(|| {
        zbus::Error::Address("找不到 IBus 总线地址（IBUS_ADDRESS 没设，地址文件也没有）".to_owned())
    })?;
    tracing::info!(%address, "连 IBus 总线");
    let connection = Builder::address(address.as_str())?
        .serve_at(FACTORY_PATH, IBusFactory::new(client))?
        .name(COMPONENT_NAME)?
        .build()
        .await?;
    tracing::info!(name = COMPONENT_NAME, path = FACTORY_PATH, "工厂已就绪");
    Ok(connection)
}

/// IBus 总线地址。
pub fn bus_address() -> Option<String> {
    address_or_file(std::env::var("IBUS_ADDRESS").ok().as_deref())
}

/// 环境变量给了就用它，没给再去读地址文件。
///
/// 读环境变量与判断拆开，是为了测试不用改进程环境：测试线程是并行跑的，
/// 一条测试 `set_var` 的同时另一条在读，结果就看时序（同文件里显示标识那两条真撞上过）。
fn address_or_file(env: Option<&str>) -> Option<String> {
    match env {
        Some(address) if !address.is_empty() => Some(address.to_owned()),
        _ => address_from_file(),
    }
}

/// 从 `~/.config/ibus/bus/` 下的地址文件里读。
///
/// 文件名是 ibus **算出来**的：`<机器 id>-<主机>-<显示标识>`。不能扫目录随便挑一个——
/// 那里常年躺着别的输入法或上一次会话留下的陈旧文件（真机上见过 fcitx 留的 `-unix-0` 指着会话总线），
/// 按字典序挑会挑中死的那个，连上去一个信号都收不到。所以先按规则算文件名，
/// 算出来的不在才退回扫目录，且只认 `IBUS_DAEMON_PID` 还活着的。
fn address_from_file() -> Option<String> {
    let dir = bus_dir()?;
    if let Some(name) = socket_name()
        && let Some(address) = read_address(&dir.join(name))
    {
        return Some(address);
    }
    tracing::debug!("按规则算的地址文件不在，退回扫目录");
    scan_bus_dir(&dir)
}

/// 算不出文件名（或那个文件是空的）时扫整个目录：先取 daemon 还活着、真写了地址的；
/// 一个都认不出活的，再取 socket 真连得上的。
///
/// flatpak 沙箱里通常没有 `WAYLAND_DISPLAY`（清单没开 wayland socket），总走这条路，而且有两处坑（本机端到端时都踩过）：
/// - 目录里常躺着 X11 会话留下的 `…-unix-0`，两行都是空的（`IBUS_ADDRESS=`、`IBUS_DAEMON_PID=`），
///   按字典序还排在当前会话的 `…-unix-wayland-0` 前面——读出空地址就直接去连，引擎起不来；
/// - 沙箱有自己的 PID 命名空间，宿主机 daemon 的 PID 在 `/proc` 里看不到，按 PID 判活一律判死；
///   socket 倒是连得上（清单给了 `~/.cache/ibus` 与 `$XDG_RUNTIME_DIR/ibus` 的只读权限，连 socket 不用写）。
///
/// 只看 socket 文件在不在不够：ibus-daemon 退出时不删它，`~/.cache/ibus` 下常年堆着没人听的 socket，
/// 陈旧的 `…-unix-0` 指着其中一个就会被挑中（Arch 上踩过，切到青简超时）。
fn scan_bus_dir(dir: &Path) -> Option<String> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();
    files.sort();
    files
        .iter()
        .filter(|path| daemon_alive(path))
        .find_map(|path| read_address(path))
        .or_else(|| {
            files
                .iter()
                .filter_map(|path| read_address(path))
                .find(|address| socket_listening(address))
        })
}

/// `unix:path=…` 的地址有没有进程在听：真连一下，没人听的陈旧 socket 会被拒绝。
/// 抽象 socket（`unix:abstract=…`）不试，算不在。
fn socket_listening(address: &str) -> bool {
    address
        .split(',')
        .find_map(|part| part.strip_prefix("unix:path="))
        .is_some_and(|path| UnixStream::connect(path).is_ok())
}

/// ibus 算地址文件名的规矩（见 ibus 的 `ibus_get_socket_path`）：
/// Wayland 会话用 `WAYLAND_DISPLAY` 当显示标识、主机名固定 `unix`；
/// X11 会话从 `DISPLAY` 里拆出主机与显示号。青简只做 Wayland，X11 那支是给
/// 「Wayland 会话里跑着 XWayland、`WAYLAND_DISPLAY` 没传进来」这种情况兜底的。
fn socket_name() -> Option<String> {
    let machine = machine_id()?;
    let (host, display) = display_id()?;
    Some(format!("{machine}-{host}-{display}"))
}

/// `(主机名, 显示标识)`。
fn display_id() -> Option<(String, String)> {
    display_id_from(
        std::env::var("WAYLAND_DISPLAY").ok().as_deref(),
        std::env::var("DISPLAY").ok().as_deref(),
    )
}

/// 按 `WAYLAND_DISPLAY` 与 `DISPLAY` 两个值算 `(主机名, 显示标识)`。
///
/// 拆成纯函数是为了测试：原来两条测试一个 `set_var("WAYLAND_DISPLAY")`、一个 `remove_var` 它，
/// 并行跑时互相踩，pre-push 全量测试时撞上过一次（单独跑怎么都复现不出来）。
fn display_id_from(wayland: Option<&str>, display: Option<&str>) -> Option<(String, String)> {
    if let Some(wayland) = wayland.filter(|w| !w.is_empty()) {
        return Some(("unix".to_owned(), wayland.to_owned()));
    }
    let (host, rest) = display?.split_once(':')?;
    // `:0.1` 的屏幕号不进文件名
    let number = rest.split('.').next()?.to_owned();
    let host = if host.is_empty() {
        "unix".to_owned()
    } else {
        host.to_owned()
    };
    Some((host, number))
}

/// 这个地址文件记的 daemon 还活着吗。陈旧文件的 PID 早就没了。
fn daemon_alive(path: &Path) -> bool {
    let Ok(content) = std::fs::read_to_string(path) else {
        return false;
    };
    let pid = content
        .lines()
        .find_map(|line| line.strip_prefix("IBUS_DAEMON_PID="))
        .and_then(|pid| pid.trim().parse::<u32>().ok());
    match pid {
        Some(pid) => Path::new(&format!("/proc/{pid}")).is_dir(),
        // 没记 PID 就不敢说它死了
        None => true,
    }
}

/// 地址文件是几行 `键=值`，要的是 `IBUS_ADDRESS`。
fn read_address(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    // 空地址当没有：X11 会话留下的陈旧文件里就是 `IBUS_ADDRESS=` 一行空值
    content.lines().find_map(|line| {
        line.strip_prefix("IBUS_ADDRESS=")
            .map(str::trim)
            .filter(|address| !address.is_empty())
            .map(str::to_owned)
    })
}

/// 地址文件所在目录 `$XDG_CONFIG_HOME/ibus/bus`，回退 `$HOME/.config/ibus/bus`。
///
/// **两处都要找是为了 Flatpak**：沙箱里 `XDG_CONFIG_HOME` 被改成 `~/.var/app/<id>/config`，
/// 而 ibus 的地址文件是**宿主机上** ibus-daemon 写的，仍在 `~/.config/ibus/bus`（manifest 里开了只读权限）。
/// 只认前者就会在沙箱里找不到总线，症状是引擎起来就退出。
fn bus_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let configured = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(value) if PathBuf::from(&value).is_absolute() => Some(PathBuf::from(value)),
        _ => None,
    };
    [configured, home.map(|home| home.join(".config"))]
        .into_iter()
        .flatten()
        .map(|base| base.join("ibus/bus"))
        .find(|dir| dir.is_dir())
}

fn machine_id() -> Option<String> {
    for path in ["/etc/machine-id", "/var/lib/dbus/machine-id"] {
        if let Ok(id) = std::fs::read_to_string(path) {
            let id = id.trim().to_owned();
            if !id.is_empty() {
                return Some(id);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 环境变量优先：ibus-daemon 拉起引擎进程时设的就是它。
    #[test]
    fn env_address_wins() {
        assert_eq!(
            address_or_file(Some("unix:abstract=/tmp/test-ibus")).as_deref(),
            Some("unix:abstract=/tmp/test-ibus")
        );
    }

    /// 地址文件里只认 `IBUS_ADDRESS=` 那一行，别的键（IBUS_DAEMON_PID）不要。
    #[test]
    fn address_file_picks_the_right_key() {
        let dir = std::env::temp_dir().join("qingjian-ibus-address-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("machine-unix-0");
        std::fs::write(
            &path,
            "IBUS_DAEMON_PID=1234\nIBUS_ADDRESS=unix:abstract=/tmp/dbus-x\n",
        )
        .unwrap();
        assert_eq!(
            read_address(&path).as_deref(),
            Some("unix:abstract=/tmp/dbus-x")
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Wayland 会话的文件名：主机名固定 `unix`，显示标识就是 `WAYLAND_DISPLAY`；两个都有时它优先。
    #[test]
    fn wayland_display_id() {
        assert_eq!(
            display_id_from(Some("wayland-0"), Some(":0")),
            Some(("unix".to_owned(), "wayland-0".to_owned()))
        );
        // 空串当没设
        assert_eq!(
            display_id_from(Some(""), Some(":0")),
            Some(("unix".to_owned(), "0".to_owned()))
        );
    }

    /// X11 兜底：`:0` 的主机名为空要补成 `unix`，`.1` 的屏幕号不进文件名。
    #[test]
    fn x11_display_id_drops_the_screen_number() {
        assert_eq!(
            display_id_from(None, Some(":0.1")),
            Some(("unix".to_owned(), "0".to_owned()))
        );
        assert_eq!(
            display_id_from(None, Some("box:2")),
            Some(("box".to_owned(), "2".to_owned()))
        );
        assert_eq!(display_id_from(None, None), None);
    }

    /// 记着的 PID 不在了就当这个地址文件是陈旧的——真机上见过 fcitx 留的文件躺在那里，
    /// 按字典序还排在当前会话的前面。
    #[test]
    fn stale_address_files_are_recognised() {
        let dir = std::env::temp_dir().join("qingjian-ibus-stale-test");
        std::fs::create_dir_all(&dir).unwrap();
        let stale = dir.join("machine-unix-0");
        // PID 取一个几乎不可能存在的值
        std::fs::write(
            &stale,
            "IBUS_ADDRESS=unix:path=/dead\nIBUS_DAEMON_PID=4194300\n",
        )
        .unwrap();
        assert!(!daemon_alive(&stale));
        let live = dir.join("machine-unix-wayland-0");
        std::fs::write(
            &live,
            format!(
                "IBUS_ADDRESS=unix:path=/live\nIBUS_DAEMON_PID={}\n",
                std::process::id()
            ),
        )
        .unwrap();
        assert!(daemon_alive(&live), "自己这个进程当然活着");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 文件里没有那一行时不能瞎猜。
    #[test]
    fn address_file_without_the_key_is_none() {
        let dir = std::env::temp_dir().join("qingjian-ibus-address-empty");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("machine-unix-0");
        std::fs::write(&path, "IBUS_DAEMON_PID=1234\n").unwrap();
        assert!(read_address(&path).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 写了 `IBUS_ADDRESS=` 但值是空的，也不能当地址用。
    #[test]
    fn blank_address_is_none() {
        let dir = std::env::temp_dir().join(format!("qingjian-ibus-blank-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("machine-unix-0");
        std::fs::write(&path, "IBUS_ADDRESS=\nIBUS_DAEMON_PID=\n").unwrap();
        assert!(read_address(&path).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 真机上的现场：两行都空的 `…-unix-0` 按字典序排在当前会话的 `…-unix-wayland-0` 前面，
    /// 扫目录要跳过它、取到后面那个。
    #[test]
    fn scan_skips_blank_address_files() {
        let dir = std::env::temp_dir().join(format!("qingjian-ibus-scan-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("machine-unix-0"),
            "IBUS_ADDRESS=\nIBUS_DAEMON_PID=\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("machine-unix-wayland-0"),
            format!(
                "IBUS_ADDRESS=unix:path=/live\nIBUS_DAEMON_PID={}\n",
                std::process::id()
            ),
        )
        .unwrap();
        assert_eq!(scan_bus_dir(&dir).as_deref(), Some("unix:path=/live"));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// flatpak 沙箱里的现场：宿主机 daemon 的 PID 在沙箱里看不到（按 PID 判活判死），
    /// 这时认 socket 真连得上的那个地址；排在前面、指着没人听的旧 socket 的不认。
    #[test]
    fn scan_falls_back_to_listening_socket() {
        let dir =
            std::env::temp_dir().join(format!("qingjian-ibus-sandbox-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let socket = dir.join("dbus-live");
        let _listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
        // daemon 退出后留下的：socket 文件还在，没人听
        let stale = dir.join("dbus-stale");
        drop(std::os::unix::net::UnixListener::bind(&stale).unwrap());
        std::fs::write(
            dir.join("machine-unix-0"),
            format!(
                "IBUS_ADDRESS=unix:path={},guid=old\nIBUS_DAEMON_PID=4194301\n",
                stale.display()
            ),
        )
        .unwrap();
        // PID 取一个几乎不可能存在的值，演「沙箱里看不到」
        std::fs::write(
            dir.join("machine-unix-wayland-0"),
            format!(
                "IBUS_ADDRESS=unix:path={},guid=abc\nIBUS_DAEMON_PID=4194300\n",
                socket.display()
            ),
        )
        .unwrap();
        assert_eq!(
            scan_bus_dir(&dir),
            Some(format!("unix:path={},guid=abc", socket.display()))
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
