//! 日志初始化：终端走 stderr；设置了 QINGJIAN_LOG_DIR 时再按天滚动写文件。

use std::io;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

/// 返回的 guard 要活到进程结束，否则文件日志会丢尾巴。
pub fn init() -> Result<Option<WorkerGuard>, io::Error> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let stderr = tracing_subscriber::fmt::layer()
        .with_writer(io::stderr)
        .with_target(false)
        .compact();
    let (file_layer, guard) = match std::env::var("QINGJIAN_LOG_DIR")
        .ok()
        .filter(|d| !d.trim().is_empty())
    {
        Some(dir) => {
            std::fs::create_dir_all(&dir)?;
            let (writer, guard) = tracing_appender::non_blocking(tracing_appender::rolling::daily(
                dir,
                "qingjian.log",
            ));
            (
                Some(
                    tracing_subscriber::fmt::layer()
                        .with_writer(writer)
                        .with_ansi(false),
                ),
                Some(guard),
            )
        }
        None => (None, None),
    };
    tracing_subscriber::registry()
        .with(filter)
        .with(stderr)
        .with(file_layer.map(Layer::boxed))
        .init();
    Ok(guard)
}
