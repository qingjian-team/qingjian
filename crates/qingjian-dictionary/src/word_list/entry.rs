//! 词表的一条：编码与原样词在 [`super::WordList`] 共用文字缓冲里的位置，加词频。

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// 原样词与编码相同时（绝大多数）两段指向同一处，不另存。16 字节、无填充，`.qj` 里原样存放。
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub(super) struct Entry {
    pub code_start: u32,

    pub word_start: u32,

    pub frequency: u32,

    pub code_len: u16,

    pub word_len: u16,
}
