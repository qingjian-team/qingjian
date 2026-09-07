//! 用 LLM 批量生成中文词的英文 / 日文释义表（词性 + 译词 + 日文假名读音），以及给多音字词标拼音。
//!
//! `generate` 从一元词频表挑词、分批问模型，结果逐行追加到 JSONL（中断了再跑会跳过已完成的词）；
//! `export` 把 JSONL 转成输入法加载的 `glossary-en.tsv` / `glossary-ja.tsv`；
//! `pinyin` 给词表里的词标拼音（建词库时含多音字的词靠它定读音），结果 `pinyin-llm.jsonl`；
//! `english` / `export-english` 给英文词写中文释义，导出 `glossary-zh.tsv`（英文候选右侧显示）。
//! 密钥来自 `--api-key` 或环境变量 `QINGJIAN_API_KEY`（也读当前目录的 `.env`）。

mod args;
mod batch;
mod client;
mod english;
mod entry;
mod error;
mod generate;
mod pinyin;
mod prompt;
mod store;
mod words;

use clap::Parser;

use crate::args::{Args, Command};
use crate::error::GlossError;

fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    if let Err(error) = run() {
        tracing::error!(%error, "失败");
        std::process::exit(1);
    }
}

fn run() -> Result<(), GlossError> {
    let args = Args::parse();
    match args.command {
        Command::Generate(generate) => {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            runtime.block_on(generate::run(generate))
        }
        Command::Export(export) => store::export(&export.input, &export.out_dir),
        Command::English(english) => {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            runtime.block_on(english::run(english))
        }
        Command::ExportEnglish(export) => store::export_english(&export.input, &export.out_dir),
        Command::Pinyin(pinyin) => {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            runtime.block_on(pinyin::run(pinyin))
        }
    }
}
