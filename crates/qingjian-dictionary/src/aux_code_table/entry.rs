//! 码表的落盘条目：一个词与它的一个码。

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// 码表里的一条：一个词与它的一个码。12 字节、无填充，原样落盘。
///
/// 条目按词文本的字节序升序排列，同一个词的条目相邻；`HASH` 索引指向这段区间的起点。
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub(super) struct CodeEntry {
    /// 词文本在词 arena 里的字节偏移。
    pub word_start: u32,

    /// 码在码 arena 里的字节偏移。
    pub code_start: u32,

    /// 词文本的字节长度。
    pub word_len: u16,

    /// 码的字节长度（1–8）。
    pub code_len: u8,

    /// 对齐用，全零。
    pub reserved: u8,
}
