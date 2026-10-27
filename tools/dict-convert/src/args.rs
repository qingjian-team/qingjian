use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use crate::mmh::MmhReferenceOptions;
use crate::stroke::StrokeOptions;

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
    /// 青简基础词库：从「输入法字词库_分类整理版」数据包 + Unihan 读音建 dict.tsv（两遍跑，见模块文档）。
    /// 各项输入缺省都并入，缺文件就报错；不要哪项用对应的 `--no-…` 排除（第一遍还没有语料词频：`--no-frequency`）
    Lexicon {
        /// 数据包目录（含 01_characters / 02_common / 03_domains），随仓库放在 assets/lexicon
        #[arg(long, default_value = "assets/lexicon")]
        pack: PathBuf,

        /// Unihan_Readings.txt
        #[arg(long, default_value = "data/unihan/Unihan_Readings.txt")]
        unihan: PathBuf,

        /// LLM 标注的多音字词读音（`gloss-gen pinyin` 的 JSONL）
        #[arg(long, default_value = "data/generated/pinyin-llm.jsonl")]
        pinyin: PathBuf,

        /// 不用 LLM 标注的读音，多音字词按 Unihan 猜
        #[arg(long)]
        no_pinyin: bool,

        /// 语料词频（lm-unigram.tsv）
        #[arg(long, default_value = "data/generated/lm-unigram.tsv")]
        frequency: PathBuf,

        /// 不用语料词频，按排序号 / 文档频次给底值（第一遍还没有语言模型时）
        #[arg(long)]
        no_frequency: bool,

        /// 把仍靠猜读音的多音字词写到这个文件（一行一个），交给 `gloss-gen pinyin`
        #[arg(long)]
        emit_ambiguous: Option<PathBuf>,

        /// 额外并入的词（`词\t拼音\t次数`，拼音可空，可给多个）：词库里没有的按次数当词频加进去，读音同领域词。
        /// 缺省是 assets/lexicon 的挖词、短语、品牌、领域、中英混杂五份；给了就只用给的
        #[arg(
            long,
            default_values = [
                "assets/lexicon/mined_words.tsv",
                "assets/lexicon/phrases.tsv",
                "assets/lexicon/brand.tsv",
                "assets/lexicon/domain_words.tsv",
                "assets/lexicon/mixed_words.tsv",
            ]
        )]
        extra_words: Vec<PathBuf>,

        /// 一份额外词也不并入
        #[arg(long)]
        no_extra_words: bool,

        /// 领域词在语料里出现不少于这个次数就留在基础词库，否则拆到 dicts/<领域>.qj
        #[arg(long, default_value_t = 50)]
        domain_keep_min: u64,

        /// 青简词库全量（数字声调版 `词\t拼音\t权重`）：已有的词补上声调，没有的换算词频，已清洗的写 dict-cleaned.tsv、其余写 dict-general.tsv
        /// 与 dicts/general.qj（都不进语言模型分词）。交付目录在仓库外，软链成 data/lexicon
        #[arg(long, default_value = "data/lexicon/qingjian-full.tone-numbers.tsv")]
        general: PathBuf,

        /// 青简词库已清洗的子集（同格式，只看第一列），这些词与基础词库同一档
        #[arg(
            long,
            default_value = "data/lexicon/subsets/curated-machine-draft.tone-numbers.tsv"
        )]
        cleaned: PathBuf,

        /// 不并入青简词库（全量与已清洗子集都不要）
        #[arg(long)]
        no_general: bool,
    },

    /// 单字声调表：通用词表（带数字声调）→ tones.tsv（`字\t带调音节…`），编进 Core 给注音按声调排序。
    /// 生成后放到 `crates/qingjian-core/data/tones.tsv`（`--out-dir crates/qingjian-core/data`）
    Tones {
        /// 通用词表（`word\tpinyin\t…`，首行表头）
        #[arg(
            long,
            default_value = "assets/lexicon/02_common/modern_chinese_common_words.tsv"
        )]
        words: PathBuf,
    },

    /// 拆字表：開放詞典網拆字字典 + 词库读音 → chaizi.tsv（`字\t读音\t部件拼音串`），编进 Core 给问字模式按部件找字。
    /// 生成后放到 `crates/qingjian-core/data/chaizi.tsv`（`--out-dir crates/qingjian-core/data`）
    Chaizi {
        /// 拆字字典（`字\t拆法…`，拆法里部件空格分隔）
        #[arg(long, default_value = "assets/chaizi/chaizi-jt.txt")]
        chaizi: PathBuf,

        /// 青简词库（`词\t音节\t词频`），取单字读音与字频
        #[arg(long, default_value = "assets/lexicon/dict.tsv")]
        dictionary: PathBuf,
    },

    /// 形码码表（五笔）：Rime `.dict.yaml` → `词\t编码\t词频`。词频由青简词库按词面回填，不用码表自带的权重
    Wubi {
        /// 输入的 Rime 码表（`.dict.yaml`，如极点 86 五笔）
        input: PathBuf,

        /// 词频来源：青简词库 TSV（`词\t拼音\t词频`）。可给多个（基础词库 + 随包领域词库），同一个词取词频最大的那份
        #[arg(long, default_value = "assets/lexicon/dict.tsv", num_args = 1..)]
        frequency: Vec<PathBuf>,

        /// 输出文件名（写在 --out-dir 下）；缺省是形码方案目录里码表的缺省名
        #[arg(long, default_value = "table.tsv")]
        name: String,
    },

    /// 五笔字根图的字形：字根表（`assets/schemes/wubi86/roots86.tsv`）+ 思源黑体 → SVG 路径表（写在 --out-dir 下）
    WubiRoots {
        /// 思源黑体 SC Regular（`SourceHanSansSC-Regular.otf`，不进仓库）
        font: PathBuf,

        /// 字根表
        #[arg(long, default_value = "assets/schemes/wubi86/roots86.tsv")]
        spec: PathBuf,

        /// 输出文件名
        #[arg(long, default_value = "roots86-glyphs.tsv")]
        name: String,

        /// 把还没挑笔画（`?`）的部件画成一张笔画编号图（PNG）
        #[arg(long)]
        sheet: Option<PathBuf>,
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

        /// 短语层文件（assets/lexicon/phrases.tsv，可给多个，人工挑的领域词 domain_words.tsv 也走这条路）：里面的词不参与分词，统计完按成分合成它们的一元 / 二元计数（见 bigram.rs 模块注释）
        #[arg(long)]
        phrases: Vec<PathBuf>,

        /// 品牌词文件（assets/lexicon/brand.tsv，可给多个，中英混杂词 mixed_words.tsv 也走这条路）：语料里没有的词按文件给的次数写进一元表。
        /// 与 --phrases 的区别：合成计数要成分词在语料里，C盘 的 C 不是语料 token，只能直接给
        #[arg(long)]
        brand: Vec<PathBuf>,

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

    /// 短语层：从 bigram 表的相邻两词与语料的相邻三词里挖 我的 / 不知道 这类人会整块打的组合 → phrases.tsv（人工过一遍后拷进 assets/lexicon/，`lexicon --extra-words` 并入）
    Phrases {
        /// 语料文件
        #[arg(required = true)]
        corpus: Vec<PathBuf>,

        /// 对话语料（corpus 里的一个）：短语在它里面的次数也要够 min_count，维基模板句进不来
        #[arg(long, default_value = "data/corpus/lccc.txt")]
        dialogue: PathBuf,

        /// 分词与成分读音用的词库（青简 TSV，同目录 dicts/ 一并读）
        #[arg(long, default_value = "data/generated/dict.tsv")]
        dict: PathBuf,

        /// 上一次的 phrases.tsv：词库已并入短语时重跑要给，里面的词先从分词词表里摘掉（否则 我的 是一个词，挖不出 我 + 的）
        #[arg(long)]
        refresh: Option<PathBuf>,

        /// 次数下限
        #[arg(long, default_value_t = 2000)]
        min_count: u32,

        /// 最多几个字
        #[arg(long, default_value_t = 4)]
        max_chars: usize,
    },

    /// 笔画表：CNS11643 全字庫「筆順資料」+ 大陆序覆盖表 → `codes/stroke.tsv`（随包笔画码表的源数据，见模块文档）。
    /// 参数的 clap 定义在 `stroke::StrokeOptions`，加参数只动那一处
    Stroke(StrokeOptions),

    /// 笔画对照表：从 hanzi-writer-data（Make Me a Hanzi；Arphic 许可，不进仓库）生成 `stroke --verify`
    /// 用的两张开发期对照表到 `data/mmh/`（笔画数、首笔几何类别，见模块文档与 assets/stroke/README.md）
    MmhReference(MmhReferenceOptions),

    /// 把 TSV 打包成 `.qj` 容器（mmap 直接用，启动近零耗时）：`dict` 读 dict.tsv 写 dict.qj，`lm` 读 lm-unigram/bigram.tsv 写 lm.qj，
    /// `glossary --language en` 读 glossary-en.tsv 写 glossary-en.qj；`model` 把导出的三件套目录（缺省 data/models/hanzhang-zhiwei）
    /// 打成一个 .qjm（`--out-dir data/models/hanzhang-zhiwei` 就写回原目录，随包只带这一个文件）；
    /// `codes` 是唯一不「原样落盘」的一种：读笔画表与词库，按取码规则算成本地码表 codes/stroke.qj（见 codes 模块）
    Pack {
        /// 打包哪种数据
        kind: PackKind,

        /// 输入文件；`dict` 一个或几个 TSV（几个合成一本；缺省 dict.tsv，有 dict-cleaned.tsv 就一起），`lm` 两个（一元表、二元表），`model` 一个目录。缺省从输出目录里找同名 TSV（`model` 缺省 data/models/hanzhang-zhiwei）
        #[arg(long, num_args = 1..)]
        input: Vec<PathBuf>,

        /// `codes` 用：笔画表（`stroke` 子命令的产物，`字\t序列`）；缺省 <输出目录>/codes/stroke.tsv
        #[arg(long)]
        stroke: Option<PathBuf>,

        /// `codes` 用：取码用的词库（`.qj` 或 TSV）；缺省 <输出目录>/dict.qj
        #[arg(long)]
        dict: Option<PathBuf>,

        /// `codes` 或 `model` 用：输出文件；模型缺省 <输出目录>/model.qjm
        #[arg(long)]
        output: Option<PathBuf>,

        /// 元数据：名称（`codes` 缺省「笔画」，别的种类必填）
        #[arg(long, default_value = "")]
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

        /// `glossary` 专用：释义表的语言代码（en / ja / zh / es / vi / de），决定输出文件名 glossary-<语言>.qj
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

    /// 本地整句模型（三件套目录 → .qjm）
    Model,

    /// 笔画码表（笔画表 + 词库 → codes/stroke.qj，随包原生码表）
    Codes,
}

impl PackKind {
    /// 子命令里写的名字（报错文案用）。
    pub fn name(self) -> &'static str {
        match self {
            Self::Dict => "dict",
            Self::Lm => "lm",
            Self::Glossary => "glossary",
            Self::Model => "model",
            Self::Codes => "codes",
        }
    }
}
