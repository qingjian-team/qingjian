//! 看门进程：青简被强行结束（崩溃、被杀、更新时被结束）时替它收掉 llama-server，不留下占着内存的进程。
//!
//! 它一直读一根管道，青简这头开着不写；青简一退出，系统关掉管道，它读到头就结束 llama-server。
//! macOS 与 Linux 用 `/bin/sh` 的 `read` 与 `kill`，Windows 用 `cmd` 的 `set /p` 与 `taskkill`。
//! 正常停服务时先结束看门进程，再结束 llama-server：免得它去结束一个已被系统另派用场的进程号。

use std::io;
use std::process::{Child, Command, Stdio};

pub(crate) struct Watchdog {
    /// 看门进程；标准输入的管道由它带着，放下就断。
    child: Child,
}

impl Watchdog {
    /// 守着进程号为 `pid` 的 llama-server。
    pub fn guard(pid: u32) -> io::Result<Self> {
        let child = command(pid)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(Self { child })
    }

    /// 看门进程自己结束了没有。管道还开着时它不该结束；结束了说明它已经动手（或这个平台上靠不住）。
    pub fn fired(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(Some(_)))
    }
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        // 强行结束：不给它读到管道尽头、再去结束 llama-server 的机会
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(unix)]
fn command(pid: u32) -> Command {
    let mut command = Command::new("/bin/sh");
    command
        .arg("-c")
        .arg("read line; kill \"$1\" 2>/dev/null")
        .arg("qingjian-watchdog")
        .arg(pid.to_string());
    command
}

#[cfg(windows)]
fn command(pid: u32) -> Command {
    use std::os::windows::process::CommandExt;

    let shell = std::env::var_os("ComSpec").unwrap_or_else(|| "cmd.exe".into());
    let mut command = Command::new(shell);
    // 整段原样交给 cmd：引号里的 & 与 > 由 cmd 解释，不经 Rust 的参数转义
    command.raw_arg(format!(
        "/d /q /c \"set /p line= & %SystemRoot%\\System32\\taskkill.exe /f /pid {pid} >nul 2>&1\""
    ));
    command.creation_flags(super::CREATE_NO_WINDOW);
    command
}

#[cfg(all(test, unix))]
mod tests {
    use std::process::{Child, Command};
    use std::time::{Duration, Instant};

    use super::Watchdog;

    /// 代替 llama-server 的长命进程。
    fn sleeper() -> Child {
        Command::new("sleep").arg("30").spawn().unwrap()
    }

    /// 最多等几秒看它退没退。
    fn exits_soon(child: &mut Child) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if matches!(child.try_wait(), Ok(Some(_))) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    }

    #[test]
    fn closing_the_pipe_ends_the_guarded_process() {
        let mut server = sleeper();
        let mut watchdog = Watchdog::guard(server.id()).unwrap();
        // 青简退出时系统关掉的就是这根管道
        drop(watchdog.child.stdin.take());
        assert!(exits_soon(&mut server));
        assert!(exits_soon(&mut watchdog.child));
    }

    #[test]
    fn stopping_the_watchdog_leaves_the_process_alone() {
        let mut server = sleeper();
        let mut watchdog = Watchdog::guard(server.id()).unwrap();
        assert!(!watchdog.fired());
        drop(watchdog);
        std::thread::sleep(Duration::from_millis(200));
        assert!(matches!(server.try_wait(), Ok(None)));
        server.kill().unwrap();
        server.wait().unwrap();
    }
}
