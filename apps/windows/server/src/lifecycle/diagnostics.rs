//! 起止记录同步写入独立日志，不依赖 tracing 的异步队列。

use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

use super::{ExitStatus, journal::Journal, panic_log, timestamp_unix_ms};

pub(super) struct Diagnostics {
    journal: Journal,

    started: Instant,
}

impl Diagnostics {
    pub(super) fn start(directory: Option<PathBuf>) -> Self {
        let diagnostics = Self {
            journal: Journal::new(directory),
            started: Instant::now(),
        };
        panic_log::install(diagnostics.journal.clone());
        diagnostics.record(format!(
            "timestamp_unix_ms={} pid={} event=process_started version={}\n",
            timestamp_unix_ms(),
            std::process::id(),
            env!("CARGO_PKG_VERSION")
        ));
        diagnostics
    }

    pub(super) fn stopped(&self, status: &ExitStatus) {
        self.record(format!(
            "timestamp_unix_ms={} pid={} event=process_stopped exit_code={} reason={} uptime_ms={}\n",
            timestamp_unix_ms(),
            std::process::id(),
            status.code,
            status.reason,
            self.started.elapsed().as_millis()
        ));
    }

    fn record(&self, record: String) {
        if let Err(error) = self.journal.record(&record) {
            let _ = writeln!(
                std::io::stderr().lock(),
                "pid={} lifecycle_record_failed kind={:?}",
                std::process::id(),
                error.kind()
            );
        }
    }
}
