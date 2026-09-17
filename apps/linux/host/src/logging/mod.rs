//! 输入法进程没有终端，日志只写文件：`$XDG_STATE_HOME/qingjian/logs/qingjian.log.<日期>`
//! （缺省 `~/.local/state/qingjian/logs`）。按天分文件，只留最近 [`KEEP_DAYS`] 天；
//! 结构照搬 macOS 壳的 logging 模块，平台差异只有目录约定。

mod log_file;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use qingjian_platform::LogLevel;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry, reload};

pub use log_file::LogFile;

/// 日志文件名前缀，后面跟 `.YYYY-MM-DD`。
pub const FILE_PREFIX: &str = "qingjian.log";

/// 保留最近几天的日志。
pub const KEEP_DAYS: i32 = 7;

/// 运行中改级别用的句柄（配置 `[general] log_level` 热切换）。
static FILTER: OnceLock<reload::Handle<EnvFilter, Registry>> = OnceLock::new();

/// 环境变量 `RUST_LOG` 给了过滤器就以它为准，配置里的级别不再生效（开发时用）。
static ENV_OVERRIDE: AtomicBool = AtomicBool::new(false);

/// 返回的 guard 要活到进程结束，否则文件日志会丢尾巴。启动时按 info 记，配置加载后再按配置切。
pub fn init() -> Option<WorkerGuard> {
    let dir = log_dir()?;
    std::fs::create_dir_all(&dir).ok()?;
    prune(&dir, jiff::Zoned::now().date());
    let (writer, guard) = tracing_appender::non_blocking(LogFile::new(dir));
    let from_env = EnvFilter::try_from_default_env().ok();
    ENV_OVERRIDE.store(from_env.is_some(), Ordering::Relaxed);
    let (filter, handle) =
        reload::Layer::new(from_env.unwrap_or_else(|| filter_for(LogLevel::Info)));
    // init() 在「全局 subscriber 已被设过」时会 panic；本函数在 qj_init 的 FFI 边界上跑，
    // panic 会越界 abort 整个 fcitx5——用 try_init 把这种情况消化成「不接管日志」。
    if tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(writer)
                .with_ansi(false),
        )
        .try_init()
        .is_err()
    {
        return None;
    }
    let _ = FILTER.set(handle);
    Some(guard)
}

/// 切日志级别。`RUST_LOG` 环境变量在时不动。
pub fn set_level(level: LogLevel) {
    if ENV_OVERRIDE.load(Ordering::Relaxed) {
        return;
    }
    if let Some(handle) = FILTER.get()
        && let Err(error) = handle.reload(filter_for(level))
    {
        tracing::warn!(%error, "切换日志级别失败");
    }
}

/// 各级别的过滤器。
fn filter_for(level: LogLevel) -> EnvFilter {
    match level {
        LogLevel::Info => EnvFilter::new("info"),
        LogLevel::Debug => EnvFilter::new("debug"),
    }
}

/// 日志目录（XDG state 约定）。
pub fn log_dir() -> Option<PathBuf> {
    // 空串也当没设（与 paths.rs 的 XDG 判法一致），否则日志目录成了相对路径，写进 fcitx5 的 cwd。
    if let Some(state) = std::env::var_os("XDG_STATE_HOME")
        && !state.is_empty()
    {
        return Some(PathBuf::from(state).join("qingjian/logs"));
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state/qingjian/logs"))
}

/// 删掉目录里日期早于 `today - KEEP_DAYS` 的日志文件；文件名解析不出日期的不动。
pub fn prune(dir: &Path, today: jiff::civil::Date) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let Ok(oldest_kept) = today.checked_sub(jiff::Span::new().days(KEEP_DAYS - 1)) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(date) = name
            .to_str()
            .and_then(|n| n.strip_prefix(FILE_PREFIX))
            .and_then(|rest| rest.strip_prefix('.'))
            .and_then(|date| date.parse::<jiff::civil::Date>().ok())
        else {
            continue;
        };
        if date < oldest_kept {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prune_keeps_recent_files_and_unrelated_names() {
        let dir = std::env::temp_dir().join("qingjian-linux-log-prune-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for name in [
            "qingjian.log.2026-09-04",
            "qingjian.log.2026-08-29",
            "qingjian.log.2026-08-28",
            "qingjian.log.bogus",
            "notes.txt",
        ] {
            std::fs::write(dir.join(name), "x").unwrap();
        }
        prune(&dir, jiff::civil::date(2026, 9, 4));
        let mut left: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(
            left,
            [
                "notes.txt",
                "qingjian.log.2026-08-29",
                "qingjian.log.2026-09-04",
                "qingjian.log.bogus"
            ]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
