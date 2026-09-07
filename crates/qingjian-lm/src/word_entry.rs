use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// 词表里的一条：词在 arena 里的位置与一元计数。下标就是词的编号。12 字节、无填充，原样落盘。
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct WordEntry {
    /// 在 `BigramModel::words` 里的字节偏移。
    pub text_start: u32,

    /// 一元计数。
    pub count: u32,

    /// 字节长度。
    pub text_len: u16,

    /// 对齐用，全零。
    pub reserved: u16,
}
