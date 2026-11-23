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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_stays_eight_bytes() {
        assert_eq!(std::mem::size_of::<Successor>(), 8);
    }

    #[test]
    fn reads_back_from_the_mapped_bytes() {
        let bytes = [1_u8, 0, 0, 0, 2, 0, 0, 0];
        let successor = Successor::read_from_bytes(&bytes).unwrap();
        assert_eq!(successor.word, 1);
        assert_eq!(successor.count, 2);
        assert_eq!(successor.as_bytes(), bytes);
    }
}
