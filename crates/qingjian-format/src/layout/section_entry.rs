use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// 分节表里的一条：标签、正文在文件里的偏移与长度。24 字节。
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct SectionEntry {
    /// 4 字节 ASCII 标签（`META`、`TEXT`……）。
    pub tag: [u8; 4],

    /// 对齐用，全零。
    pub reserved: u32,

    /// 正文起始的文件偏移，8 字节对齐。
    pub offset: u64,

    /// 正文长度（不含对齐填充）。
    pub length: u64,
}

impl SectionEntry {
    pub const SIZE: usize = size_of::<Self>();
}
