use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "qingjian-dict-convert",
    about = "把第三方词库 / 词典转换成青简的 TSV，或把 TSV 打包成 .qj"
)]
pub struct Args {
    /// 输出目录
    #[arg(long, default_value = "data/generated")]
    pub out_dir: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// 青简基础词库：从「输入法字词库_分类整理版」数据包 + Unihan 读音建 dict.tsv（两遍跑，见模块文档）
    Lexicon {
        /// 数据包目录（含 01_characters / 02_common / 03_domains），随仓库放在 assets/lexicon
        #[arg(long, default_value = "assets/lexicon")]
        pack: PathBuf,

        /// Unihan_Readings.txt
        #[arg(long, default_value = "data/unihan/Unihan_Readings.txt")]
        unihan: PathBuf,

        /// LLM 标注的多音字词读音（`gloss-gen pinyin` 的 JSONL）
        #[arg(long)]
        pinyin: Option<PathBuf>,

        /// 语料词频（lm-unigram.tsv）；没有就按排序号 / 文档频次给底值
        #[arg(long)]
        frequency: Option<PathBuf>,

        /// 把仍靠猜读音的多音字词写到这个文件（一行一个），交给 `gloss-gen pinyin`
        #[arg(long)]
        emit_ambiguous: Option<PathBuf>,

        /// 额外并入的词（`词\t次数`，`mine` 挖出来的 oov-candidates.tsv，可给多个）：词库里没有的按次数当词频加进去，读音同领域词
        #[arg(long)]
        extra_words: Vec<PathBuf>,

        /// 领域词在语料里出现不少于这个次数就留在基础词库，否则拆到 dicts/<领域>.qj
        #[arg(long, default_value_t = 50)]
        domain_keep_min: u64,
    },

    /// CC-CEDICT `cedict_ts.u8` → glossary-en.tsv
    Cedict {
        /// 输入文件
        input: PathBuf,
    },

    /// 英文词表（每行 `词\t编码[\t…]`，带不带表头都行，比如 `assets/lexicon/05_english/00_all_words.tsv`）→ english.tsv
    English {
        /// 输入文件
        #[arg(required = true)]
        inputs: Vec<PathBuf>,

        /// 词频表（`编码\t词频`，`tools/corpus/english_frequency.py` 生成）；给了就写进第三列，前缀补全按它排
        #[arg(long)]
        frequency: Option<PathBuf>,
    },

    /// Unicode CLDR emoji annotations（`annotations/<语言>/annotations.json`、`annotationsDerived/…`）→ emoji-<语言>.tsv：`词\temoji …`
    Emoji {
        /// 输入的 JSON 文件
        #[arg(required = true)]
        inputs: Vec<PathBuf>,

        /// 语言代码，决定输出文件名（zh → emoji-zh.tsv，en → emoji-en.tsv）
        #[arg(long, default_value = "zh")]
        language: String,
    },

    /// 纯文本语料（每行一段）→ lm-unigram.tsv + lm-bigram.tsv：按词库分词后统计词级一元 / 二元计数
    Bigram {
        /// 语料文件（UTF-8 纯文本，简体）
        #[arg(required = true)]
        corpus: Vec<PathBuf>,

        /// 分词用的词库（青简 TSV）；同目录 dicts/ 下的领域词库会一并用于分词（词表与拆分前一致）
        #[arg(long, default_value = "data/generated/dict.tsv")]
        dict: PathBuf,

        /// 计数低于此值的二元组不输出
        #[arg(long, default_value_t = 3)]
        min_count: u32,

        /// 最多输出多少条二元组（按计数取前 N）
        #[arg(long, default_value_t = 3_000_000)]
        max_bigrams: usize,
    },

    /// 从语料里挖词库没收的词：分词时被拆成连续单字的段按子串计数，出现够多的写到 oov-candidates.tsv（再交给 gloss-gen pinyin 标音、lexicon --extra-words 并入）
    Mine {
        /// 语料文件（UTF-8 纯文本，简体）；给了 --candidates 就不用扫语料
        #[arg(required_unless_present = "candidates")]
        corpus: Vec<PathBuf>,

        /// 语言模型一元表（`词\t次数`），算相邻字对 PMI 用
        #[arg(long, default_value = "data/generated/lm-unigram.tsv")]
        frequency: PathBuf,

        /// 相邻字对 PMI 的下限；0 不过滤
        #[arg(long, default_value_t = 3.0)]
        min_pmi: f64,

        /// 跳过扫语料，直接过滤上一次写出的 oov-candidates.tsv（调阈值用）
        #[arg(long)]
        candidates: Option<PathBuf>,

        /// 分词用的词库（青简 TSV）
        #[arg(long, default_value = "assets/lexicon/dict.tsv")]
        dict: PathBuf,

        /// 出现次数低于此值的不要
        #[arg(long, default_value_t = 200)]
        min_count: u32,

        /// 最多几个字
        #[arg(long, default_value_t = 4)]
        max_chars: usize,
    },

    /// 把 TSV 打包成 `.qj` 容器（mmap 直接用，启动近零耗时）：`dict` 读 dict.tsv 写 dict.qj，`lm` 读 lm-unigram/bigram.tsv 写 lm.qj，
    /// `glossary --language en` 读 glossary-en.tsv 写 glossary-en.qj
    Pack {
        /// 打包哪种数据
        kind: PackKind,

        /// 输入文件；`dict` 一个 TSV，`lm` 两个（一元表、二元表）。缺省从输出目录里找同名 TSV
        #[arg(long, num_args = 1..)]
        input: Vec<PathBuf>,

        /// 元数据：名称
        #[arg(long)]
        name: String,

        /// 元数据：许可证（SPDX 标识，如 GPL-3.0-only、CC-BY-SA-4.0）
        #[arg(long, default_value = "")]
        license: String,

        /// 元数据：署名 / 版权行
        #[arg(long, default_value = "")]
        attribution: String,

        /// 元数据：来源 URL
        #[arg(long, default_value = "")]
        source: String,

        /// 元数据：数据版本（上游版本号或日期）
        #[arg(long, default_value = "")]
        data_version: String,

        /// `glossary` 专用：释义表的语言代码（en / ja / zh），决定输出文件名 glossary-<语言>.qj
        #[arg(long, default_value = "en")]
        language: String,
    },
}

/// `pack` 能打的数据种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum PackKind {
    /// 拼音词库
    Dict,

    /// 词级 bigram 语言模型
    Lm,

    /// 释义表（glossary-<语言>.tsv → glossary-<语言>.qj）
    Glossary,
}
