//! 把数据源转换成青简的 TSV 格式，或打包成 `.qj`。
//!
//! - `lexicon`：青简基础词库，「输入法字词库_分类整理版」数据包（规范字 / 常用词 / THUOCL 领域词）+ Unihan 读音 + LLM 多音字标注 → `dict.tsv`
//! - `cedict`：CC-CEDICT（CC BY-SA 4.0）→ `glossary-en.tsv`（释义表的备用来源，现在用 gloss-gen 的 LLM 表）
//! - `english`：`词\t编码` 英文词表（数据包的 `05_english`，ESDB / CSpell，MIT）→ `english.tsv`
//! - `emoji`：Unicode CLDR annotations（Unicode License v3，`--language zh|en`）→ `emoji-<语言>.tsv`（可发布，放 `assets/emoji/`）
//! - `bigram`：纯文本语料（如 `tools/corpus/parquet_to_text.py` 转出的中文维基 CC BY-SA 4.0、LCCC 对话 MIT）→ `lm-unigram.tsv` + `lm-bigram.tsv`
//! - `mine`：语料里分词落成连续单字的段 → `oov-candidates.tsv`（词库没收的高频词，标音后用 `lexicon --extra-words` 并入）
//! - `pack dict|lm|glossary`：TSV → `.qj` 容器（`dict.qj` / `lm.qj`），带名称 / 许可证 / 署名元数据，输入法与 CLI 优先加载它
//!
//! 输出默认写到仓库根目录 `data/generated/`（gitignore）。

mod args;
mod bigram;
mod cedict;
mod emoji;
mod english;
mod error;
mod lexicon;
mod oov_filter;
mod pack;

use clap::Parser;
use tracing_subscriber::EnvFilter;

use crate::args::{Args, Command};
use crate::error::ConvertError;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), ConvertError> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();
    let args = Args::parse();
    std::fs::create_dir_all(&args.out_dir)?;
    match args.command {
        Command::Lexicon {
            pack,
            unihan,
            pinyin,
            frequency,
            emit_ambiguous,
            extra_words,
            domain_keep_min,
        } => lexicon::convert(
            &pack,
            &unihan,
            pinyin.as_deref(),
            frequency.as_deref(),
            emit_ambiguous.as_deref(),
            &extra_words,
            domain_keep_min,
            &args.out_dir,
        ),
        Command::Cedict { input } => cedict::convert(&input, &args.out_dir.join("glossary-en.tsv")),
        Command::English { inputs, frequency } => english::convert(
            &inputs,
            frequency.as_deref(),
            &args.out_dir.join("english.tsv"),
        ),
        Command::Emoji { inputs, language } => emoji::convert(
            &inputs,
            &args.out_dir.join(format!("emoji-{language}.tsv")),
            &language,
        ),
        Command::Bigram {
            corpus,
            dict,
            min_count,
            max_bigrams,
        } => bigram::convert(&corpus, &dict, min_count, max_bigrams, &args.out_dir),
        Command::Mine {
            corpus,
            dict,
            min_count,
            max_chars,
            frequency,
            min_pmi,
            candidates,
        } => bigram::mine(
            &bigram::MineOptions {
                corpus,
                dict,
                min_count,
                max_chars,
                frequency,
                min_pmi,
                candidates,
            },
            &args.out_dir,
        ),
        Command::Pack {
            kind,
            input,
            name,
            license,
            attribution,
            source,
            data_version,
            language,
        } => pack::pack(
            kind,
            &input,
            &language,
            qingjian_format::Metadata {
                name,
                license,
                attribution,
                source,
                version: data_version,
                ..qingjian_format::Metadata::default()
            },
            &args.out_dir,
        ),
    }
}
