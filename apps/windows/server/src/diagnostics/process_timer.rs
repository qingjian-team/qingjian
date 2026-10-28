//! 保留标准时间戳，在每条 tracing 事件前缀加入当前进程号。

use std::fmt;

use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::{FormatTime, SystemTime};

pub(crate) struct ProcessTimer;

impl FormatTime for ProcessTimer {
    fn format_time(&self, writer: &mut Writer<'_>) -> fmt::Result {
        SystemTime.format_time(writer)?;
        write!(writer, " pid={}", std::process::id())
    }
}

#[cfg(test)]
mod tests {
    use super::ProcessTimer;

    #[test]
    fn every_event_contains_the_process_id() {
        let path = std::env::temp_dir().join(format!(
            "qingjian-process-timer-{}-{}.log",
            std::process::id(),
            super::super::timestamp_unix_ms()
        ));
        let subscriber = tracing_subscriber::fmt()
            .with_timer(ProcessTimer)
            .with_ansi(false)
            .with_writer(std::fs::File::create(&path).unwrap())
            .finish();
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!("first event");
            tracing::warn!("second event");
        });
        let text = std::fs::read_to_string(&path).unwrap();
        let pid = format!("pid={}", std::process::id());
        assert_eq!(text.lines().count(), 2);
        assert!(text.lines().all(|line| line.contains(&pid)));
        std::fs::remove_file(path).unwrap();
    }
}
