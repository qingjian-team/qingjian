//! 配置来源快照和进程日志标识；不改变配置读取或输入处理。

mod config_source;
mod process_timer;

pub(super) use config_source::ConfigSource;
pub(super) use process_timer::ProcessTimer;

#[cfg(test)]
fn timestamp_unix_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
}
