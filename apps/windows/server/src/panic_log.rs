//! 使用现有 server tracing 输出记录 panic 位置，不记录 payload。

pub(super) fn install() {
    std::panic::set_hook(Box::new(|info| {
        let location = info.location();
        tracing::error!(
            pid = std::process::id(),
            file = location.map(|location| location.file()),
            line = location.map(|location| location.line()),
            backtrace = %std::backtrace::Backtrace::force_capture(),
            "Server thread panicked; payload omitted"
        );
    }));
}

#[cfg(test)]
mod tests {
    #[test]
    fn existing_log_omits_panic_payload() {
        let path = std::env::temp_dir().join(format!("qingjian-panic-{}.log", std::process::id()));
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(std::fs::File::create(&path).unwrap())
            .finish();
        let previous = std::panic::take_hook();
        super::install();
        tracing::subscriber::with_default(subscriber, || {
            let _ = std::panic::catch_unwind(|| panic!("PRIVATE_PAYLOAD_913478"));
        });
        std::panic::set_hook(previous);
        let log = std::fs::read_to_string(&path).unwrap();
        assert!(log.contains("payload omitted"));
        assert!(!log.contains("PRIVATE_PAYLOAD_913478"));
        std::fs::remove_file(path).unwrap();
    }
}
