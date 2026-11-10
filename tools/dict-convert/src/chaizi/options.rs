//! `chaizi` 子命令的参数与默认值。clap 定义就在这里（`Command::Chaizi` 装整个结构体），加参数只动这一处。

use std::path::PathBuf;

use clap::Args;

/// `chaizi` 子命令的参数。
#[derive(Debug, Args)]
pub struct ChaiziOptions {
    /// 拆字字典（`字<TAB>拆法…`，一种拆法里部件用空格分隔）：简体表在前、繁体表在后。
    /// 繁体表里「简体表没有的字」按繁简索引落到简体主键，拆法并进同一个字
    #[arg(
        long,
        num_args = 1..,
        value_delimiter = ',',
        default_value = "../qingjian-dicts/official/upstream/chaizi/chaizi-jt.txt,../qingjian-dicts/official/upstream/chaizi/chaizi-ft.txt"
    )]
    pub chaizi: Vec<PathBuf>,

    /// 青简词库（`词<TAB>音节<TAB>词频`），取单字读音与字频
    #[arg(
        long,
        default_value = "../qingjian-dicts/official/lexicon/zh/base/words.tsv"
    )]
    pub dictionary: PathBuf,

    /// 繁简索引（`繁体<TAB>简体`）：繁体字典里的字先落到简体主键
    #[arg(
        long,
        default_value = "../qingjian-dicts/official/upstream/chaizi/fanjian_suoyin.txt"
    )]
    pub fanjian: PathBuf,
}
