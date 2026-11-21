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
        let end = offset.checked_add(len)?;
        let bytes = map.get(offset..end)?;
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
                // 映射文件可能被别的进程改写（O_TRUNC 还会让访问直接触发 SIGBUS），
                // 构造时校验过的结论不能一直当真；这里重新校验，坏字节就退化成空串。
                // 代价是 O(len)，但 Text 目前只用于整段小文本，不在热路径的逐词取值上。
                std::str::from_utf8(&map[*offset..*offset + *len]).unwrap_or_default()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// 打开时校验过 UTF-8，不代表之后映射内容不会变：别的进程就地改写后，
    /// `deref` 不能再凭旧结论返回一个非法的 `&str`。
    #[test]
    fn mutated_mapping_is_revalidated() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("qingjian-text-{}-{nanos}.bin", std::process::id()));
        std::fs::write(&path, "hello").unwrap();
        let file = std::fs::File::open(&path).unwrap();
        let map = Arc::new(unsafe { Mmap::map(&file).unwrap() });
        let text = Text::mapped(Arc::clone(&map), 0, 5).unwrap();
        assert_eq!(&*text, "hello");
        let mut writer = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        writer.write_all(&[0xff, 0xfe, 0xfd, 0xfc, 0xfb]).unwrap();
        writer.flush().unwrap();
        assert_eq!(&*text, "");
        drop(map);
        drop(text);
        let _ = std::fs::remove_file(path);
    }
}
