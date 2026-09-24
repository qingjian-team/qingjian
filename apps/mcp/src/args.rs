//! MCP 服务器启动参数：数据文件覆盖项（缺省与 CLI 同一套自动探测）。

use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "qingjian-mcp",
    about = "青简 MCP Server：查词 / 释义工具，走 stdio"
)]
pub struct Args {
    /// 词库路径（TSV 或 .qj）。缺省自动探测 data/generated 或 assets/lexicon/dict.tsv
    #[arg(long)]
    pub dict: Option<PathBuf>,

    /// 释义表路径。缺省自动探测 data/generated 或 assets/glossary/glossary-<language>.tsv
    #[arg(long)]
    pub glossary: Option<PathBuf>,

    /// 学习语言：en / ja / es / zh；与 CLI 同一语义，释义表的语言
    #[arg(long, env = "QINGJIAN_LEARNING_LANGUAGE", default_value = "en")]
    pub language: String,

    /// 英文词表路径（中英混输）。缺省自动探测 english.tsv
    #[arg(long)]
    pub english: Option<PathBuf>,
}
