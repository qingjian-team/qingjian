//! panic 只记录线程、源码位置与回溯，不调用会输出 payload 的默认 hook。

use std::backtrace::Backtrace;
use std::io::Write;

use super::{journal::Journal, timestamp_unix_ms};

pub(super) fn install(journal: Journal) {
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        let location = info.location();
        let backtrace = Backtrace::force_capture();
        // 不读取 payload，默认 hook 也不能链式调用；文件与 stderr 均只写固定元数据。
        let record = format!(
            "timestamp_unix_ms={} pid={} event=panic thread_id={:?} thread_name={:?} file={:?} line={:?} column={:?} payload_redacted=true\n{backtrace}\n",
            timestamp_unix_ms(),
            std::process::id(),
            thread.id(),
            thread.name(),
            location.map(|location| location.file()),
            location.map(|location| location.line()),
            location.map(|location| location.column())
        );
        if journal.record(&record).is_err() {
            // stderr 失败也不能再 panic，否则 hook 内二次 panic 会直接终止进程。
            let _ = writeln!(
                std::io::stderr().lock(),
                "pid={} event=panic thread_id={:?} panic_record_failed=true payload_redacted=true",
                std::process::id(),
                thread.id()
            );
        }
    }));
}
