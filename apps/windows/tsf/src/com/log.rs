//! DLL 自己的文件日志：按天一个文件 `%LOCALAPPDATA%\Qingjian\logs\tsf.<YYYY-MM-DD>.log`，与 Server / 设置程序同目录，
//! 只留最近 [`KEEP_DAYS`] 天（命名与清理规则在 `qingjian_platform::logs::daily`）。
//! 不走 tracing 全局订阅器（宿主进程可能已装了自己的）；任何失败都吞掉，日志不能拖垮宿主。
//! 每次都开文件追加一行：DLL 被加载进每个应用进程，多进程同时追加同一天的文件，这样最省事也最稳。
//! 日期用 `GetLocalTime` 而不是时区库：这份代码跑在每一个有文本框的进程里，越轻越好。

use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use qingjian_platform::logs::daily::{self, KEEP_DAYS};
use windows::Win32::Foundation::SYSTEMTIME;
use windows::Win32::System::SystemInformation::GetLocalTime;

const PREFIX: &str = "tsf";

/// 本进程上次写日志的日子（自 1970 起的天数）；换了天才清一次旧文件。
static LAST_DAY: AtomicU32 = AtomicU32::new(0);

pub(crate) fn log(message: &str) {
    let Some(dir) = dir() else { return };
    let t = unsafe { GetLocalTime() };
    let day = day_number(&t);
    if LAST_DAY.swap(day, Ordering::Relaxed) != day {
        let _ = std::fs::create_dir_all(&dir);
        daily::prune(&dir, PREFIX, day, KEEP_DAYS);
    }
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(file_name(&t)))
    else {
        return;
    };
    let _ = writeln!(file, "{} [pid {}] {message}", now(&t), std::process::id());
}

/// 日志里怎么写一个键。可打印字符只记类别、不记是哪个，虚拟键码也不记（字母键的键码就是那个字母）：
/// 一串收键日志连起来就是用户打的内容。功能键（回车、退格、方向键、带 Ctrl 的组合）记虚拟键码，排查要用。
pub(crate) fn key_label(virtual_key: u32, character: Option<char>) -> String {
    match character.filter(|c| !c.is_control()) {
        Some(c) if c.is_alphabetic() => "letter".to_owned(),
        Some(c) if c.is_numeric() => "digit".to_owned(),
        Some(c) if c.is_whitespace() => "space".to_owned(),
        Some(_) => "symbol".to_owned(),
        None => format!("vk={virtual_key}"),
    }
}

fn file_name(t: &SYSTEMTIME) -> String {
    daily::file_name(
        PREFIX,
        i64::from(t.wYear),
        u32::from(t.wMonth),
        u32::from(t.wDay),
    )
}

fn now(t: &SYSTEMTIME) -> String {
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, t.wMilliseconds
    )
}

fn dir() -> Option<PathBuf> {
    qingjian_platform::dirs::log_dir()
}

/// 自 1970-01-01 起的天数，日期比较用。
fn day_number(t: &SYSTEMTIME) -> u32 {
    daily::days_from_civil(i64::from(t.wYear), u32::from(t.wMonth), u32::from(t.wDay))
}

#[cfg(test)]
mod tests {
    use super::key_label;

    #[test]
    fn printable_keys_are_logged_by_kind_only() {
        assert_eq!(key_label(0x41, Some('a')), "letter");
        assert_eq!(key_label(0x31, Some('1')), "digit");
        assert_eq!(key_label(0x39, Some('(')), "symbol");
        assert_eq!(key_label(0x20, Some(' ')), "space");
        assert_eq!(key_label(0x0D, Some('\r')), "vk=13");
        assert_eq!(key_label(0x25, None), "vk=37");
    }
}
