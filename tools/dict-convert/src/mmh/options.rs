//! `mmh-reference` 子命令的参数与默认值。clap 定义就在这里（`Command::MmhReference` 装整个结构体）。

use std::path::PathBuf;

use clap::Args;

/// `mmh-reference` 子命令的参数。
#[derive(Debug, Args)]
pub struct MmhReferenceOptions {
    /// hanzi-writer-data 解包目录（每字一个 `<字>.json`，含 medians）
    #[arg(long, default_value = "data/mmh/package")]
    pub mmh: PathBuf,

    /// 字表：只出表里的字，按表序排列
    #[arg(
        long,
        default_value = "../qingjian-dicts/official/upstream/01_characters/level1_3500.tsv"
    )]
    pub filter: PathBuf,

    /// 对照产物目录（gitignore，不进仓库）：写 prc-counts-l1.tsv 与 prc-first-strokes-l1.tsv
    #[arg(long, default_value = "data/mmh")]
    pub out_dir: PathBuf,
}
