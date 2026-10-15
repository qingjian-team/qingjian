//! 青简设置程序（新）：三平台共用、画在 bamboo-slip 上，读写输入法同一份配置文件，改动即时生效。
//!
//! - `qingjian-settings`：开窗口，读写用户配置（`paths::config_path`）。
//! - `--config <文件>`：换一份配置文件读写（试用时不碰真配置）。
//! - `--png <目录>`：不开窗口，拷一份配置到目录里，按脚本点几下，每步存一张截图。
#![cfg_attr(windows, windows_subsystem = "windows")]

mod docs;
mod jobs;
mod library;
mod message;
mod offscreen;
mod page_id;
mod phrase_editor;
mod search;
mod settings;
mod shortcut;
mod stats;
mod support;
mod system;
mod view;

use std::path::PathBuf;

use bamboo_slip::{WindowOptions, run_window};

use settings::Settings;
use system::paths;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();
    let result = run();
    if let Err(error) = &result {
        tracing::error!(%error, "设置程序退出");
    }
    result
}

/// 日志同时写 stderr 与输入法日志目录下的 `qingjian-settings.<日期>.log`（留 7 天）：Windows 上没有控制台，打不开时看这个文件。
fn init_logging() {
    use tracing_subscriber::fmt::writer::MakeWriterExt;
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let file = paths::log_dir().and_then(|dir| {
        tracing_appender::rolling::Builder::new()
            .rotation(tracing_appender::rolling::Rotation::DAILY)
            .filename_prefix("qingjian-settings")
            .filename_suffix("log")
            .max_log_files(7)
            .build(dir)
            .ok()
    });
    let builder = tracing_subscriber::fmt().with_env_filter(filter);
    match file {
        Some(file) => builder
            .with_ansi(false)
            .with_writer(std::io::stderr.and(file))
            .init(),
        None => builder.init(),
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = paths::config_path();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => config = args.next().map(PathBuf::from),
            "--png" => {
                let directory = args.next().map(PathBuf::from).ok_or("--png 后面要给目录")?;
                return offscreen::snapshot::run(config.as_deref(), &directory);
            }
            other => return Err(format!("不认识的参数：{other}").into()),
        }
    }
    #[cfg(windows)]
    if !system::single_instance::acquire() {
        return Ok(());
    }
    let config = config.ok_or("找不到配置文件的位置")?;
    let settings = Settings::load(config)?;
    let mut options = WindowOptions::new("青简设置", (900.0, 680.0));
    options.min_size = (640.0, 420.0);
    run_window(settings, options)?;
    Ok(())
}
