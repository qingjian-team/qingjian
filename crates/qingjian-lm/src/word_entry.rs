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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_stays_twelve_bytes() {
        assert_eq!(std::mem::size_of::<WordEntry>(), 12);
    }

    #[test]
    fn round_trips_through_the_mapped_bytes() {
        let entry = WordEntry {
            text_start: 7,
            count: 9,
            text_len: 3,
            reserved: 0,
        };
        let bytes = entry.as_bytes();
        assert_eq!(bytes.len(), 12);
        let back = WordEntry::read_from_bytes(bytes).unwrap();
        assert_eq!(back.text_start, 7);
        assert_eq!(back.count, 9);
        assert_eq!(back.text_len, 3);
        assert_eq!(back.reserved, 0);
    }
}
