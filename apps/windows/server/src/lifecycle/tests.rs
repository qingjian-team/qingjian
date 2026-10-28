//! 子进程隔离全局 panic hook，验证脱敏及受控返回时的日志落盘。

use std::path::PathBuf;
use std::process::Command;

use super::{ExitStatus, journal::Journal, panic_log};

#[test]
fn panic_hook_records_location_without_payload() {
    const CHILD: &str = "QINGJIAN_PANIC_JOURNAL_TEST_DIR";
    const SECRET: &str = "PANIC_PAYLOAD_MUST_NOT_BE_LOGGED_9d931e";
    if let Some(directory) = std::env::var_os(CHILD) {
        panic_log::install(Journal::new(Some(PathBuf::from(directory))));
        let _ = std::panic::catch_unwind(|| panic!("{SECRET}"));
        return;
    }
    let directory = scratch("panic");
    let output = child(
        "lifecycle::tests::panic_hook_records_location_without_payload",
        CHILD,
        &directory,
    );
    assert!(output.status.success(), "{output:?}");
    let record = journal_text(&directory);
    assert!(record.contains("event=panic"));
    assert!(record.contains("thread_id="));
    assert!(record.contains("tests.rs"));
    assert!(record.contains("payload_redacted=true"));
    assert!(!record.contains(SECRET));
    assert!(!String::from_utf8_lossy(&output.stdout).contains(SECRET));
    assert!(!String::from_utf8_lossy(&output.stderr).contains(SECRET));
    let fallback = child(
        "lifecycle::tests::panic_hook_records_location_without_payload",
        CHILD,
        &directory.join("missing-directory"),
    );
    assert!(fallback.status.success(), "{fallback:?}");
    assert!(!String::from_utf8_lossy(&fallback.stdout).contains(SECRET));
    let stderr = String::from_utf8_lossy(&fallback.stderr);
    assert!(stderr.contains("panic_record_failed=true"));
    assert!(!stderr.contains(SECRET));
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn controlled_return_flushes_async_log_before_stopping_record() {
    const CHILD: &str = "QINGJIAN_CONTROLLED_EXIT_TEST_DIR";
    if let Some(directory) = std::env::var_os(CHILD) {
        let directory = PathBuf::from(directory);
        let status = super::run(Some(directory.clone()), || {
            let file = std::fs::File::create(directory.join("async.log")).unwrap();
            let (writer, _guard) = tracing_appender::non_blocking(file);
            let subscriber = tracing_subscriber::fmt()
                .with_ansi(false)
                .with_writer(writer)
                .finish();
            tracing::subscriber::with_default(subscriber, || {
                tracing::error!("controlled failure before return");
            });
            ExitStatus::new(17, "test_controlled_failure")
        });
        std::process::exit(i32::from(status.code));
    }
    let directory = scratch("controlled-exit");
    let output = child(
        "lifecycle::tests::controlled_return_flushes_async_log_before_stopping_record",
        CHILD,
        &directory,
    );
    assert_eq!(output.status.code(), Some(17), "{output:?}");
    let record = journal_text(&directory);
    assert!(record.contains("event=process_started"));
    assert!(record.contains("event=process_stopped exit_code=17 reason=test_controlled_failure"));
    assert!(
        std::fs::read_to_string(directory.join("async.log"))
            .unwrap()
            .contains("controlled failure before return")
    );
    std::fs::remove_dir_all(directory).unwrap();
}

fn scratch(kind: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "qingjian-{kind}-{}-{}",
        std::process::id(),
        super::timestamp_unix_ms()
    ));
    std::fs::create_dir(&directory).unwrap();
    directory
}

fn child(name: &str, variable: &str, directory: &std::path::Path) -> std::process::Output {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env(variable, directory)
        .output()
        .unwrap()
}

fn journal_text(directory: &std::path::Path) -> String {
    let path = std::fs::read_dir(directory)
        .unwrap()
        .map(Result::unwrap)
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("server-lifecycle.day-")
        })
        .unwrap()
        .path();
    std::fs::read_to_string(path).unwrap()
}
