use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// CSR 里的一条后继：后词编号与 (前词, 后词) 计数。8 字节，原样落盘。
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct Successor {
    /// 后词编号。
    pub word: u32,

    /// 二元计数。
    pub count: u32,
}
