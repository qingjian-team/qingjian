//! DLL 自己的文件日志：按天一个文件 `%LOCALAPPDATA%\Qingjian\tsf.<YYYY-MM-DD>.log`，只留最近 [`KEEP_DAYS`] 天（与 Server 的滚动策略一致）。
//! 不走 tracing 全局订阅器（宿主进程可能已装了自己的）；任何失败都吞掉，日志不能拖垮宿主。
//! 每次都开文件追加一行：DLL 被加载进每个应用进程，多进程同时追加同一天的文件，这样最省事也最稳。

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use windows::Win32::Foundation::SYSTEMTIME;
use windows::Win32::System::SystemInformation::GetLocalTime;

/// 留几天的日志。
const KEEP_DAYS: u32 = 7;

const PREFIX: &str = "tsf.";
const SUFFIX: &str = ".log";

/// 本进程上次写日志的日子（`yyyymmdd`）；换了天才清一次旧文件。
static LAST_DAY: AtomicU32 = AtomicU32::new(0);

pub(crate) fn log(message: &str) {
    let Some(dir) = dir() else { return };
    let t = unsafe { GetLocalTime() };
    let day = day_number(&t);
    if LAST_DAY.swap(day, Ordering::Relaxed) != day {
        let _ = std::fs::create_dir_all(&dir);
        prune(&dir, day);
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

fn file_name(t: &SYSTEMTIME) -> String {
    format!(
        "{PREFIX}{:04}-{:02}-{:02}{SUFFIX}",
        t.wYear, t.wMonth, t.wDay
    )
}

fn now(t: &SYSTEMTIME) -> String {
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, t.wMilliseconds
    )
}

fn dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|base| PathBuf::from(base).join("Qingjian"))
}

/// 自 1970-01-01 起的天数，日期比较用。
fn day_number(t: &SYSTEMTIME) -> u32 {
    days_from_civil(i64::from(t.wYear), u32::from(t.wMonth), u32::from(t.wDay))
}

/// 公历日期 → 天数（Howard Hinnant 的 `days_from_civil`），够我们比大小用。
fn days_from_civil(year: i64, month: u32, day: u32) -> u32 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = (y - era * 400) as u32;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146_097 + i64::from(doe) + 719_468) as u32
}

/// 删掉目录里早于 `today - KEEP_DAYS` 的 `tsf.<日期>.log`；别的文件不碰。
fn prune(dir: &Path, today: u32) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(day) = name.to_str().and_then(log_day) else {
            continue;
        };
        if today.saturating_sub(day) >= KEEP_DAYS {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// `tsf.2026-09-11.log` → 那天的天数；不是我们的日志文件为 `None`。
fn log_day(name: &str) -> Option<u32> {
    let date = name.strip_prefix(PREFIX)?.strip_suffix(SUFFIX)?;
    let mut parts = date.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(days_from_civil(year, month, day))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_numbers_follow_the_calendar() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(
            days_from_civil(2026, 9, 11) - days_from_civil(2026, 9, 4),
            7
        );
        assert_eq!(
            days_from_civil(2026, 3, 1) - days_from_civil(2026, 2, 28),
            1
        );
    }

    #[test]
    fn only_our_dated_logs_are_recognised() {
        assert_eq!(
            log_day("tsf.2026-09-11.log"),
            Some(days_from_civil(2026, 9, 11))
        );
        assert_eq!(log_day("tsf.log"), None);
        assert_eq!(log_day("qingjian-server.2026-09-11.log"), None);
        assert_eq!(log_day("tsf.2026-13-01.log"), None);
    }
}
