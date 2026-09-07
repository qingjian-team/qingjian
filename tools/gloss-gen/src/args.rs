use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(about = "用 LLM 批量生成中文词的英文 / 日文释义表，或给多音字词标拼音")]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// 从一元词频表挑词，分批问模型，结果追加到 JSONL（可中断续跑）
    Generate(GenerateArgs),

    /// 把 JSONL 导出成 glossary-en.tsv / glossary-ja.tsv
    Export(ExportArgs),

    /// 给多音字词标拼音：词表一行一个词，结果追加到 JSONL（可中断续跑），`dict-convert lexicon --pinyin` 读它
    Pinyin(PinyinArgs),

    /// 英→中释义：给英文词表里的词写中文对应词，结果追加到 JSONL（可中断续跑）
    English(EnglishArgs),

    /// 把英→中 JSONL 导出成 glossary-zh.tsv
    ExportEnglish(ExportEnglishArgs),
}

#[derive(Debug, clap::Args)]
pub struct EnglishArgs {
    /// 英文词表（`词\t编码\t词频`）
    #[arg(long, default_value = "assets/lexicon/english.tsv")]
    pub words: PathBuf,

    /// 只要词频（wordfreq Zipf ×1000）不低于这个值的词
    #[arg(long, default_value_t = 2500)]
    pub min_frequency: u32,

    /// 这些表（带表头的 TSV，第一列是词）里的词不看词频一律要，比如技术词表
    #[arg(long, num_args = 0..)]
    pub include: Vec<PathBuf>,

    /// 只处理前 N 个（试跑用）
    #[arg(long)]
    pub limit: Option<usize>,

    /// 结果 JSONL，已有的词跳过
    #[arg(long, default_value = "assets/glossary/gloss-en-llm.jsonl")]
    pub out: PathBuf,

    /// 每个请求带几个词
    #[arg(long, default_value_t = 40)]
    pub batch: usize,

    /// 同时几个请求
    #[arg(long, default_value_t = 8)]
    pub concurrency: usize,

    /// OpenAI 兼容接口地址
    #[arg(long, default_value = "https://api.deepseek.com")]
    pub base_url: String,

    /// 模型名
    #[arg(long, default_value = "deepseek-v4-flash")]
    pub model: String,

    /// 密钥；缺省读环境变量
    #[arg(long, env = "QINGJIAN_API_KEY", hide_env_values = true)]
    pub api_key: String,

    /// 单个请求的超时秒数
    #[arg(long, default_value_t = 180)]
    pub timeout_secs: u64,
}

#[derive(Debug, clap::Args)]
pub struct ExportEnglishArgs {
    /// `english` 写出的 JSONL
    #[arg(long, default_value = "assets/glossary/gloss-en-llm.jsonl")]
    pub input: PathBuf,

    /// 输出目录，写 glossary-zh.tsv
    #[arg(long, default_value = "assets/glossary")]
    pub out_dir: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct PinyinArgs {
    /// 词表，一行一个词（`dict-convert lexicon --emit-ambiguous` 写出的）
    #[arg(long, default_value = "data/generated/lexicon-ambiguous.txt")]
    pub words: PathBuf,

    /// 结果 JSONL，已有的词跳过
    #[arg(long, default_value = "data/generated/pinyin-llm.jsonl")]
    pub out: PathBuf,

    /// 只处理前 N 个没做过的词（试跑用）
    #[arg(long)]
    pub limit: Option<usize>,

    /// 每个请求带几个词
    #[arg(long, default_value_t = 60)]
    pub batch: usize,

    /// 同时几个请求
    #[arg(long, default_value_t = 8)]
    pub concurrency: usize,

    /// OpenAI 兼容接口地址
    #[arg(long, default_value = "https://api.deepseek.com")]
    pub base_url: String,

    /// 模型名
    #[arg(long, default_value = "deepseek-v4-flash")]
    pub model: String,

    /// 密钥；缺省读环境变量
    #[arg(long, env = "QINGJIAN_API_KEY", hide_env_values = true)]
    pub api_key: String,

    /// 单个请求的超时秒数
    #[arg(long, default_value_t = 180)]
    pub timeout_secs: u64,
}

#[derive(Debug, clap::Args)]
pub struct GenerateArgs {
    /// 一元词频表（`词\t次数`），`dict-convert bigram` 生成的 lm-unigram.tsv
    #[arg(long, default_value = "data/generated/lm-unigram.tsv")]
    pub words: PathBuf,

    /// 只要出现次数不低于这个值的词
    #[arg(long, default_value_t = 200)]
    pub min_count: u64,

    /// 只要不超过这么多个字的词
    #[arg(long, default_value_t = 4)]
    pub max_chars: usize,

    /// 只处理词频最高的前 N 个（试跑用）
    #[arg(long)]
    pub limit: Option<usize>,

    /// 结果 JSONL，已有的词跳过
    #[arg(long, default_value = "data/generated/gloss-llm.jsonl")]
    pub out: PathBuf,

    /// 每个请求带几个词
    #[arg(long, default_value_t = 40)]
    pub batch: usize,

    /// 同时几个请求
    #[arg(long, default_value_t = 8)]
    pub concurrency: usize,

    /// OpenAI 兼容接口地址
    #[arg(long, default_value = "https://api.deepseek.com")]
    pub base_url: String,

    /// 模型名
    #[arg(long, default_value = "deepseek-v4-flash")]
    pub model: String,

    /// 密钥；缺省读环境变量
    #[arg(long, env = "QINGJIAN_API_KEY", hide_env_values = true)]
    pub api_key: String,

    /// 单个请求的超时秒数
    #[arg(long, default_value_t = 180)]
    pub timeout_secs: u64,
}

#[derive(Debug, clap::Args)]
pub struct ExportArgs {
    /// `generate` 写出的 JSONL
    #[arg(long, default_value = "data/generated/gloss-llm.jsonl")]
    pub input: PathBuf,

    /// 输出目录，写 glossary-en.tsv 与 glossary-ja.tsv
    #[arg(long, default_value = "data/generated")]
    pub out_dir: PathBuf,
}
