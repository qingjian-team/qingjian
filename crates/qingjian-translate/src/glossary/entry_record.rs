use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// `.qj` 里的一条：词在 arena 里的位置，以及它的释义在释义表里的一段。12 字节、无填充。
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct EntryRecord {
    /// 词的字节偏移。
    pub word_start: u32,

    /// 第一条释义在 `SenseRecord` 表里的下标。
    pub sense_start: u32,

    /// 词的字节长度。
    pub word_len: u16,

    /// 释义条数。
    pub sense_count: u16,
}
