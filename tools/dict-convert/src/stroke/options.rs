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

    /// 字表白名单（缺省通用规范字表）：只出表里的字，按表序排列；给多张就按给出的顺序拼起来，
    /// 同一个字取先出现的那次（拿简体表 + 繁体表一起跑，就得到繁简合并的笔顺表）
    #[arg(long, num_args = 1.., value_delimiter = ',', default_value = "../qingjian-dicts/official/upstream/01_characters/standard_8105.tsv")]
    pub filter: Vec<PathBuf>,

    /// 繁简索引（每行「繁体、简体」，制表符分隔）：白名单里的繁体字形并到简体主键，內/円 这类不再单独成行
    #[arg(
        long,
        default_value = "../qingjian-dicts/official/upstream/chaizi/fanjian_suoyin.txt"
    )]
    pub fanjian: PathBuf,

    /// 大陆序覆盖表：部件重写规则 + 例外字 + 整字补录
    #[arg(
        long,
        default_value = "../qingjian-dicts/official/upstream/stroke/prc-rules.tsv"
    )]
    pub prc_rules: PathBuf,

    /// 产物路径；缺省写到 <输出目录>/codes/stroke.tsv
    #[arg(long)]
    pub output: Option<PathBuf>,

    /// 序列写成键位形式（`h` 横 / `s` 竖 / `p` 撇 / `n` 点捺 / `z` 折）而不是 CNS 的 `1 2 3 5 n`：
    /// 这一份给 Core 的问字模式按笔画找字用（`data/stroke.tsv`），`pack codes` 要的是数字形式
    #[arg(long)]
    pub keys: bool,
    /// 写完再对照大陆规范：笔画数抽样比对、首笔全量比对，白名单之外一处不符就退出码非 0；
    /// 对照表找不到的那张跳过并提示（mmh-reference 生成）
    #[arg(long)]
    pub verify: bool,

    /// 抽样用的字表：按表序每 `--stride` 字取一个
    #[arg(
        long,
        default_value = "../qingjian-dicts/official/upstream/01_characters/level1_3500.tsv"
    )]
    pub sample: PathBuf,

    /// 抽样密度：每几字取一个
    #[arg(long, default_value_t = 12)]
    pub stride: usize,

    /// 笔画数对照表（`字<TAB>大陆笔画数`）：找不到就跳过笔画数对照
    #[arg(long, default_value = "data/mmh/prc-counts-l1.tsv")]
    pub reference: PathBuf,

    /// 残留差异白名单（`字<TAB>本表笔画数<TAB>对照笔画数<TAB>说明`）
    #[arg(
        long,
        default_value = "../qingjian-dicts/official/upstream/stroke/residual-whitelist.tsv"
    )]
    pub whitelist: PathBuf,

    /// 首笔对照表（`字<TAB>几何类别 h/s/p/n/z/?`）：字表全量比对；找不到就跳过首笔对照
    #[arg(long, default_value = "data/mmh/prc-first-strokes-l1.tsv")]
    pub first_reference: PathBuf,

    /// 首笔残留差异白名单（`字<TAB>本表首笔<TAB>对照类别<TAB>说明`）
    #[arg(
        long,
        default_value = "../qingjian-dicts/official/upstream/stroke/residual-first-strokes.tsv"
    )]
    pub first_whitelist: PathBuf,
}
