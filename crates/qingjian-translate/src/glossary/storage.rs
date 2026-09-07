use std::collections::HashMap;

use qingjian_core::Translation;
use qingjian_format::{Table, Text};

use super::entry_record::EntryRecord;
use super::sense_record::SenseRecord;

/// 释义表的两种存法：TSV 解析出来的哈希表，或映射的 `.qj`（arena + 定长表 + 文件里的哈希索引）。
#[derive(Debug)]
pub enum Storage {
    /// 内存里自己的。
    Owned(HashMap<String, Translation>),

    /// 映射文件里的。
    Mapped {
        /// 所有字符串首尾相接。
        text: Text,

        /// 词条表，下标即编号。
        entries: Table<EntryRecord>,

        /// 释义表。
        senses: Table<SenseRecord>,

        /// 词 → 编号的哈希索引。
        index: Table<u32>,
    },
}
