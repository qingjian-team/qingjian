//! 同步追加完整记录；按 UTC 日编号轮转，只清理本模块七天以前的文件。

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::timestamp_unix_ms;

const DAY_MS: u128 = 86_400_000;
const RETAIN_DAYS: u128 = 7;
const PREFIX: &str = "server-lifecycle.day-";
const SUFFIX: &str = ".log";

#[derive(Clone)]
pub(super) struct Journal {
    directory: Option<PathBuf>,
}

impl Journal {
    pub(super) fn new(directory: Option<PathBuf>) -> Self {
        Self { directory }
    }

    pub(super) fn record(&self, record: &str) -> io::Result<()> {
        self.record_on_day(timestamp_unix_ms() / DAY_MS, record)
    }

    fn record_on_day(&self, day: u128, record: &str) -> io::Result<()> {
        let directory = self
            .directory
            .as_deref()
            .ok_or_else(|| io::Error::other("log directory unavailable"))?;
        prune(directory, day);
        // 每次重新选择日期，不持有跨日文件；长跑进程也不会继续写已过保留期的文件。
        let path = directory.join(format!("{PREFIX}{day}{SUFFIX}"));
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        append(&mut file, record)
    }
}

fn append(file: &mut File, record: &str) -> io::Result<()> {
    // 先完整构造记录，再追加，避免多进程的字段或 panic 行逐次写入时相互穿插。
    file.write_all(record.as_bytes())?;
    file.flush()?;
    file.sync_data()
}

fn prune(directory: &Path, today: u128) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    let first_retained = today.saturating_sub(RETAIN_DAYS - 1);
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(day) = name
            .to_str()
            .and_then(|name| name.strip_prefix(PREFIX))
            .and_then(|name| name.strip_suffix(SUFFIX))
            .and_then(|day| day.parse::<u128>().ok())
        else {
            continue;
        };
        if day < first_retained && entry.file_type().is_ok_and(|kind| kind.is_file()) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Journal, PREFIX};

    #[test]
    fn rollover_keeps_seven_days_and_ignores_unrelated_files() {
        let directory = std::env::temp_dir().join(format!(
            "qingjian-journal-{}-{}",
            std::process::id(),
            super::super::timestamp_unix_ms()
        ));
        std::fs::create_dir(&directory).unwrap();
        let journal = Journal::new(Some(directory.clone()));
        let unrelated = directory.join("server-lifecycle.123.log");
        std::fs::write(&unrelated, "do not delete").unwrap();
        for day in 1..=10 {
            journal.record_on_day(day, "event=test\n").unwrap();
        }
        assert!(unrelated.is_file());
        for day in 1..=3 {
            assert!(!directory.join(format!("{PREFIX}{day}.log")).exists());
        }
        for day in 4..=10 {
            assert!(directory.join(format!("{PREFIX}{day}.log")).is_file());
        }
        // 已存在的长跑对象也转向新日期，不重新创建清理过的旧文件。
        journal
            .record_on_day(11, "event=process_stopped\n")
            .unwrap();
        assert!(!directory.join(format!("{PREFIX}4.log")).exists());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
