//! 拆字：问字模式下按部件读音找字，`umumu` → 林、`uyanshe` → 谢、`uniuniuniu` → 犇。
//!
//! 表是 `data/chaizi.tsv`（`qingjian-dict-convert chaizi` 从開放詞典網拆字字典 CC BY 3.0 与青简词库生成，不手改），
//! 编进 Core。部件读音连写对上整串的排前面，敲到一半的按前缀接着出；同一档按字频。

mod table;

use std::sync::LazyLock;

pub use table::ChaiziTable;

static TABLE: LazyLock<ChaiziTable> =
    LazyLock::new(|| ChaiziTable::parse(include_str!("../../data/chaizi.tsv")));

/// 部件读音连写为 `keys`（或以它开头）的字与它最常见的读音，最多 `limit` 个。
pub fn lookup(keys: &str, limit: usize) -> Vec<(char, &'static str)> {
    TABLE.lookup(keys, limit)
}

/// 有没有字的部件读音连写以 `keys` 开头：问字模式据此判断数字键是选字还是码点的一部分。
pub fn has_match(keys: &str) -> bool {
    !TABLE.lookup(keys, 1).is_empty()
}
