//! 青简 MCP Server 入口。
//!
//! 走 stdio 与 MCP 客户端对话（Claude Desktop / 编辑器等），首版暴露两个工具：
//! - `lookup`：拼音输入串 → 候选 + 释义（与输入法同一排序）
//! - `gloss`：词 → 学习语言译文
//!
//! 启动：`cargo run -p qingjian-mcp --`（在仓库根，让数据文件自动探测命中）。

mod args;
mod engine;
mod error;
mod protocol;
mod server;
mod tools;

use std::io;

use clap::Parser;
use tracing_subscriber::EnvFilter;

use crate::args::Args;

fn main() {
    // MCP 协议走 stdout，日志必须去 stderr，否则会污染帧流
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();
    if let Err(error) = run() {
        tracing::error!(%error, "qingjian-mcp 退出");
        std::process::exit(1);
    }
}

fn run() -> Result<(), error::Error> {
    let args = Args::parse();
    let engine = engine::build_engine(&args)?;
    tracing::info!("青简 MCP Server 就绪，等待 stdio 帧");
    server::run(tools::Tools::new(engine))
}
