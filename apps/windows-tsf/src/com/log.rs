//! DLL 自包含的文件日志：直接追加写 `%LOCALAPPDATA%\Qingjian\tsf.log`，不经 tracing 的进程级全局
//! 订阅器（那个在宿主进程已被占用时会静默失败）。DLL 被加载进任意应用进程，这是唯一可靠的观测手段。
//!
//! 每次调用打开 / 追加 / 关闭一次文件——量很小（按键级），换来的是「不管在谁的进程里都写得进」。
//! 任何失败都静默吞掉：日志绝不能拖垮宿主进程。

use std::io::Write;
use std::path::PathBuf;

use windows::Win32::System::SystemInformation::GetLocalTime;

/// 追加一行日志（自动带本地时间戳与进程 id）。失败静默。
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

/// 本地时间戳 `YYYY-MM-DD HH:MM:SS.mmm`。
fn now() -> String {
    // SAFETY: GetLocalTime 只写出一个 SYSTEMTIME，无入参。
    let t = unsafe { GetLocalTime() };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, t.wMilliseconds
    )
}

/// 日志目录：`%LOCALAPPDATA%\Qingjian`。
fn dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|base| PathBuf::from(base).join("Qingjian"))
}
