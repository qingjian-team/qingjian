//! 拆字：问字模式下按部件读音、按笔顺、或两者混着找字。
//!
//! ```text
//! umumu        林      部件读音
//! uhspn        木      整字笔顺（h 横 s 竖 p 撇 n 点捺 z 折）
//! umuhspn      林      木(mu) + 木的笔顺
//! uheszh       和      禾(he) + 口(szh)
//! ukoumu       困      囗 认 kou（部件的俗名，见 parts.tsv）
//! ```
//!
//! 拆字表与笔顺表是生成物，部件别名表手工维护，都编进 Core：
//! - `data/chaizi.tsv`：`字\t读音\t拆法部件\t拆法读音`（`qingjian-dict-convert chaizi`）；
//! - `data/stroke.tsv`：`字\t笔顺`，笔顺是 `[1235n]+`，`qingjian-dict-convert stroke` 出；
//! - `data/parts.tsv`：部件别名（俗名读音、CNS 没有的笔画符号），就在这里手工维护，格式见 `data/README.md`。
//!
//! 查询是「输入串 × 每种拆法」的对齐：每个部件要么吃掉它的一个读音，要么吃掉它笔顺的前几笔，
//! 吃完的部分可以停在任意位置——敲到一半就出候选。笔画键只有 `h` 横 / `s` 竖 / `p` 撇 /
//! `n` 点捺 / `z` 折五个，与大陆厂商的习惯一致；整串都是笔画键时再走一遍整字笔顺表（正序与倒序）。

mod align;
mod table;

use std::sync::LazyLock;

pub use table::{Hit, Tier};

static TABLE: LazyLock<table::ChaiziTable> = LazyLock::new(|| {
    table::ChaiziTable::parse(
        include_str!("../../data/chaizi.tsv"),
        include_str!("../../data/stroke.tsv"),
        include_str!("../../data/parts.tsv"),
    )
});

/// 按 `body` 找字，最多 `limit` 个，按「吃得最多的、切得最省的、刚好对上的」排在前面。
/// `body` 为空时没有结果。
///
/// `prefer_traditional` 跟界面的简繁设置走：简体设置时繁体字典来的拆法（`瞭` 的 `目尞` 落到 `了` 上）
/// 排在简体拆法后面，繁体设置反过来。
pub fn lookup(body: &str, limit: usize) -> Vec<Hit> {
    TABLE.lookup(body, limit)
}

/// 有没有字能被 `body` 对上：问字模式据此判断数字键是选字还是码点的一部分。
///
/// **只问「部件读音连写」这一条老口径**（`umumu`），不含笔画与混输——`uface` 这类十六进制串
/// 要是被笔画解释成有候选，数字键就再也进不了码点输入（`hex_looking_letters_select_instead_of_typing_a_code_point`）。
/// 笔画与混输的候选照常出，只是不参与这个判定。
pub fn has_match(keys: &str) -> bool {
    TABLE.has_reading_match(keys)
}
