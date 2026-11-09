//! 把数据源转换成青简的 TSV 格式，或打包成 `.qj`。
//!
//! - `lexicon`：青简基础词库，「输入法字词库_分类整理版」数据包（规范字 / 常用词 / THUOCL 领域词）+ Unihan 读音 + LLM 多音字标注 → `dict.tsv`
//! - `tones`：通用词表的数字声调 → 单字声调表 `tones.tsv`（编进 Core，注音按敲的声调排序）
//! - `chaizi`：開放詞典網拆字字典（CC BY 3.0）+ 词库读音 → 拆字表 `chaizi.tsv`（编进 Core，问字模式按部件读音找字）
//! - `wubi`：Rime 形码码表（极点 86 五笔，Apache-2.0）→ 形码方案的 `table.tsv`（`词\t编码\t词频`，词频由青简词库按词面回填）
//! - `wubi-roots`：五笔字根表（`assets/schemes/wubi86/roots86.tsv`）+ 思源黑体（SIL OFL，不进仓库）→ `roots86-glyphs.tsv`（字根图的 SVG 路径，随包，沿用 OFL）
//! - `cedict`：CC-CEDICT（CC BY-SA 4.0）→ `glossary-en.tsv`（释义表的备用来源，现在用 gloss-gen 的 LLM 表）
//! - `english`：`词\t编码` 英文词表（数据包的 `05_english`，ESDB / CSpell，MIT）→ `english.tsv`
//! - `emoji`：Unicode CLDR annotations（Unicode License v3，`--language zh|en`）→ `emoji-<语言>.tsv`（可发布，放 `assets/emoji/`）
//! - `bigram`：纯文本语料（如 `tools/corpus/parquet_to_text.py` 转出的中文维基 CC BY-SA 4.0、LCCC 对话 MIT）→ `lm-unigram.tsv` + `lm-bigram.tsv`
//! - `mine`：语料里分词落成连续单字的段 → `oov-candidates.tsv`（词库没收的高频词，标音后用 `lexicon --extra-words` 并入）
//! - `phrases`：bigram 表的相邻两词 + 语料的相邻三词 → `phrases.tsv`（我的 / 不知道 这类短语层，读音由成分词拼出，同样用 `lexicon --extra-words` 并入）
//! - `stroke`：CNS11643 全字庫「筆順資料」+ 大陆序覆盖表（`assets/stroke/prc-rules.tsv`）→ `codes/stroke.tsv`（随包笔画码表的源数据，`--verify` 抽样对照大陆笔画数）
//! - `mmh-reference`：hanzi-writer-data（Make Me a Hanzi；Arphic 许可，不进仓库）→ `data/mmh/` 的两张开发期对照表
//!   （笔画数、首笔几何类别），`stroke --verify` 找不到哪张就跳过哪张对照
//! - `pack codes`：笔画表（`stroke` 的产物，`字\t序列`）+ 词库 → `codes/stroke.qj`（随包原生辅码表：单字前 4 笔 + 末笔、
//!   词组每字首笔；缺字的词跳过并计入统计，见 `codes` 模块）
//! - `pack dict|lm|glossary|model`：TSV → `.qj` 容器（`dict.qj` / `lm.qj`），带名称 / 许可证 / 署名元数据，输入法与 CLI 优先加载它；
//!   `model` 把本地整句模型的三件套目录打成一个 `.qjm` 文件
//!
//! 输出默认写到仓库根目录 `data/generated/`（gitignore）。

mod args;
mod bigram;
mod cedict;
mod chaizi;
mod codes;
mod emoji;
mod english;
mod error;
mod lexicon;
mod mmh;
mod oov_filter;
mod pack;
mod phrases;
mod stroke;
mod wubi;
mod wubi_roots;

use std::path::PathBuf;

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
            no_pinyin,
            frequency,
            no_frequency,
            emit_ambiguous,
            extra_words,
            no_extra_words,
            domain_keep_min,
            general,
            cleaned,
            no_general,
        } => {
            let pinyin = included(
                pinyin,
                no_pinyin,
                "generate it with gloss-gen pinyin (assets/lexicon/QINGJIAN.md), or pass --no-pinyin",
            )?;
            let frequency = included(
                frequency,
                no_frequency,
                "run bigram first; pass --no-frequency on the first pass",
            )?;
            let extra_words = if no_extra_words {
                Vec::new()
            } else {
                extra_words
                    .into_iter()
                    .map(|path| included(path, false, "restore it, list the files with --extra-words, or pass --no-extra-words"))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .flatten()
                    .collect()
            };
            let hint = "symlink the Qingjian lexicon delivery as data/lexicon (or pass --general / --cleaned), or pass --no-general";
            let general = included(general, no_general, hint)?;
            let cleaned = included(cleaned, no_general, hint)?;
            lexicon::convert(
                &pack,
                &unihan,
                pinyin.as_deref(),
                frequency.as_deref(),
                emit_ambiguous.as_deref(),
                &extra_words,
                domain_keep_min,
                general.as_deref(),
                cleaned.as_deref(),
                &args.out_dir,
            )
        }
        Command::Tones { words } => {
            let characters = lexicon::char_tones(&words, &args.out_dir.join("tones.tsv"))?;
            tracing::info!(characters, "单字声调表已写出");
            Ok(())
        }
        Command::Chaizi { chaizi, dictionary } => {
            let summary = chaizi::convert(&chaizi, &dictionary, &args.out_dir.join("chaizi.tsv"))?;
            tracing::info!(
                characters = summary.characters,
                unread = summary.unread,
                "拆字表已写出"
            );
            Ok(())
        }
        Command::WubiRoots {
            font,
            spec,
            name,
            sheet,
        } => {
            let summary =
                wubi_roots::run(&font, &spec, &args.out_dir.join(name), sheet.as_deref())?;
            tracing::info!(
                roots = summary.roots,
                pending = summary.pending,
                "五笔字根字形已导出"
            );
            Ok(())
        }
        Command::Wubi {
            input,
            frequency,
            name,
        } => {
            let converted = wubi::convert(&input, &frequency, &args.out_dir.join(name))?;
            tracing::info!(
                entries = converted.entries,
                from_corpus = converted.with_frequency,
                unknown = converted.unknown,
                "形码码表已写出"
            );
            Ok(())
        }
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
            phrases,
            brand,
            min_count,
            max_bigrams,
        } => bigram::convert(
            &corpus,
            &dict,
            &phrases,
            &brand,
            min_count,
            max_bigrams,
            &args.out_dir,
        ),
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
        Command::Phrases {
            corpus,
            dialogue,
            dict,
            refresh,
            min_count,
            max_chars,
        } => phrases::mine(
            &phrases::PhraseOptions {
                dict,
                refresh,
                corpus,
                dialogue,
                min_count,
                max_chars,
            },
            &args.out_dir,
        ),
        Command::Stroke(options) => stroke::convert(&options, &args.out_dir),
        Command::MmhReference(options) => mmh::generate(&options),
        Command::Pack {
            kind,
            input,
            stroke,
            dict,
            output,
            name,
            license,
            attribution,
            source,
            data_version,
            language,
        } => pack::pack(
            kind,
            &input,
            &pack::CodePaths {
                stroke: stroke.as_deref(),
                dict: dict.as_deref(),
                output: output.as_deref(),
            },
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

/// 缺省并入的输入：排除了是 `None`，没排除但文件不在就报错（不悄悄少并一份）。
fn included(
    path: PathBuf,
    excluded: bool,
    hint: &'static str,
) -> Result<Option<PathBuf>, ConvertError> {
    if excluded {
        return Ok(None);
    }
    if !path.is_file() {
        return Err(ConvertError::MissingInput { path, hint });
    }
    Ok(Some(path))
}
