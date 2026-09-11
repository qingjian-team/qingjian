//! DLL 自己的文件日志，追加写 `%LOCALAPPDATA%\Qingjian\tsf.log`。
//! 不走 tracing 全局订阅器（宿主进程可能已装了自己的）；任何失败都吞掉，日志不能拖垮宿主。

use std::io::Write;
use std::path::PathBuf;

use windows::Win32::System::SystemInformation::GetLocalTime;

pub(crate) fn log(message: &str) {
    let Some(dir) = dir() else { return };
    let _ = std::fs::create_dir_all(&dir);
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("tsf.log"))
    else {
        return;
    };
    let _ = writeln!(file, "{} [pid {}] {message}", now(), std::process::id());
}

fn now() -> String {
    let t = unsafe { GetLocalTime() };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, t.wMilliseconds
    )
}

fn dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|base| PathBuf::from(base).join("Qingjian"))
}
