use std::ops::Deref;
use std::sync::Arc;

use memmap2::Mmap;

/// 一段 UTF-8 文本（arena）：从 TSV 解析出来时是自己的 `String`，从 `.qj` 打开时是映射文件里的一段，
/// 打开时校验过一次 UTF-8。对外都是 `&str`。
#[derive(Debug)]
pub enum Text {
    /// 内存里自己的。
    Owned(String),

    /// 映射文件里 `offset..offset + len` 的字节。
    Mapped {
        /// 整个文件的映射，多个分节共享。
        map: Arc<Mmap>,

        /// 正文起始偏移。
        offset: usize,

        /// 正文字节数。
        len: usize,
    },
}

impl Default for Text {
    fn default() -> Self {
        Self::Owned(String::new())
    }
}

impl Text {
    /// 从映射里取一段并校验 UTF-8。
    pub(crate) fn mapped(map: Arc<Mmap>, offset: usize, len: usize) -> Option<Self> {
        let bytes = map.get(offset..offset + len)?;
        std::str::from_utf8(bytes).ok()?;
        Some(Self::Mapped { map, offset, len })
    }
}

impl Deref for Text {
    type Target = str;

    fn deref(&self) -> &str {
        match self {
            Self::Owned(text) => text,
            Self::Mapped { map, offset, len } => {
                let bytes = &map[*offset..*offset + *len];
                // SAFETY: `Text::mapped` 构造时对这段字节做过 UTF-8 校验；数据文件只整体替换（写临时文件再改名）、
                // 从不就地修改，映射内容在生命周期内不变。这里不能再用带校验的版本：每次取 `&str` 都扫一遍几十 MB 的 arena。
                unsafe { std::str::from_utf8_unchecked(bytes) }
            }
        }
    }
}
