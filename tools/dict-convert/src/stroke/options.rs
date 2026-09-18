//! stroke 子命令的参数与默认值。clap 定义就在这里（`Command::Stroke` 装整个结构体），加参数只动这一处。

use std::path::PathBuf;

use clap::Args;

/// `stroke` 子命令的参数。
#[derive(Debug, Args)]
pub struct StrokeOptions {
    /// CNS 筆順資料（`CNS_strokes_sequence.txt`）：`CNS 字碼<TAB>1-5 序列`
    #[arg(long, default_value = "data/cns/CNS_strokes_sequence.txt")]
    pub cns_seq: PathBuf,

    /// CNS→Unicode 对照表（`CNS2UNICODE_Unicode*.txt`）：给文件或目录（目录取其中的对照表）
    #[arg(long, default_value = "data/cns", num_args = 1..)]
    pub cns_map: Vec<PathBuf>,

    /// 官方筆畫數（`CNS_stroke.txt`）：与序列长度自洽的字才留，不给就不过滤
    #[arg(long)]
    pub cns_count: Option<PathBuf>,

    /// 自洽过滤的容差：序列长度与筆畫數之差超过它的字丢掉
    #[arg(long, default_value_t = 1)]
    pub max_diff: usize,

    /// 字表白名单（缺省通用规范字表）：只出表里的字，按表序排列
    #[arg(long, default_value = "assets/lexicon/01_characters/standard_8105.tsv")]
    pub filter: PathBuf,

    /// 大陆序覆盖表：部件重写规则 + 例外字 + 整字补录
    #[arg(long, default_value = "assets/stroke/prc-rules.tsv")]
    pub prc_rules: PathBuf,

    /// 产物路径；缺省写到 <输出目录>/codes/stroke.tsv
    #[arg(long)]
    pub output: Option<PathBuf>,

    /// 写完再按抽样对照表比对大陆笔画数，白名单之外一处不符就退出码非 0
    #[arg(long)]
    pub verify: bool,

    /// 抽样用的字表：按表序每 `--stride` 字取一个
    #[arg(long, default_value = "assets/lexicon/01_characters/level1_3500.tsv")]
    pub sample: PathBuf,

    /// 抽样密度：每几字取一个
    #[arg(long, default_value_t = 12)]
    pub stride: usize,

    /// 对照表（`字<TAB>大陆笔画数`）：抽样字表里每个字都要有
    #[arg(long, default_value = "tools/dict-convert/testdata/prc-counts-l1.tsv")]
    pub reference: PathBuf,

    /// 残留差异白名单（`字<TAB>本表笔画数<TAB>对照笔画数<TAB>说明`）
    #[arg(long, default_value = "assets/stroke/residual-whitelist.tsv")]
    pub whitelist: PathBuf,
}
