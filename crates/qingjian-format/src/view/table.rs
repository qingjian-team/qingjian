use std::ops::Deref;
use std::sync::Arc;

use memmap2::Mmap;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// 一段定长结构体数组：从 TSV 解析出来时是自己的 `Vec`，从 `.qj` 打开时是映射文件里的一段。
/// 两者对外都是 `&[T]`（`Deref`），查询代码不区分。
#[derive(Debug)]
pub enum Table<T> {
    /// 内存里自己的。
    Owned(Vec<T>),

    /// 映射文件里 `offset..offset + len` 的字节，打开时已校验对齐与长度。
    Mapped {
        /// 整个文件的映射，多个分节共享。
        map: Arc<Mmap>,

        /// 正文起始偏移。
        offset: usize,

        /// 正文字节数。
        len: usize,
    },
}

impl<T> Default for Table<T> {
    fn default() -> Self {
        Self::Owned(Vec::new())
    }
}

impl<T: FromBytes + Immutable + KnownLayout> Table<T> {
    /// 从映射里取一段，校验对齐与整除。
    pub(crate) fn mapped(map: Arc<Mmap>, offset: usize, len: usize) -> Option<Self> {
        let bytes = map.get(offset..offset + len)?;
        <[T]>::ref_from_bytes(bytes).ok()?;
        Some(Self::Mapped { map, offset, len })
    }
}

impl<T: FromBytes + Immutable + KnownLayout> Deref for Table<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        match self {
            Self::Owned(items) => items,
            Self::Mapped { map, offset, len } => {
                <[T]>::ref_from_bytes(&map[*offset..*offset + *len])
                    .expect("mapped table was validated when the container was opened")
            }
        }
    }
}

impl<T: FromBytes + IntoBytes + Immutable + KnownLayout> Table<T> {
    /// 落盘用的字节形式。
    pub fn as_bytes(&self) -> &[u8] {
        let items: &[T] = self;
        items.as_bytes()
    }
}
