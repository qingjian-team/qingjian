//! Server 的同步起止与脱敏 panic 记录；不推断未留下记录的退出原因。

mod diagnostics;
mod exit_status;
mod journal;
mod panic_log;

#[cfg(test)]
mod tests;

use diagnostics::Diagnostics;
pub(super) use exit_status::ExitStatus;

pub(super) fn run(
    directory: Option<std::path::PathBuf>,
    run: impl FnOnce() -> ExitStatus,
) -> ExitStatus {
    let diagnostics = Diagnostics::start(directory);
    // 调用返回前，run 内的异步日志 guard 已析构；最后的起止记录不依赖它的队列。
    let status = run();
    diagnostics.stopped(&status);
    status
}

pub(super) fn timestamp_unix_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
