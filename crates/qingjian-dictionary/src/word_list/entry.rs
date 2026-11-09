//! 英文词表的一条：编码与原样词在 [`super::WordList`] 共用文字缓冲里的位置，加词频。

/// 原样词与编码相同时（绝大多数）两段指向同一处，不另存。
#[derive(Debug, Clone, Copy)]
pub(super) struct Entry {
    pub code_start: u32,

    pub code_len: u16,

    pub word_start: u32,

    pub word_len: u16,

    pub frequency: u32,
}
