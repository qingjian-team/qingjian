use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use memmap2::Mmap;
use zerocopy::{FromBytes, Immutable, KnownLayout};

use crate::{
    FORMAT_VERSION, FormatError, Header, Kind, MAGIC, META_TAG, Metadata, SECTION_ALIGN,
    SectionEntry, Table, Text,
};

/// 打开的 `.qj` 文件：整个文件一次 mmap，分节按标签取。
#[derive(Debug)]
pub struct Container {
    /// 文件映射，交给取出去的 [`Table`] / [`Text`] 共享。
    map: Arc<Mmap>,

    /// 数据种类。
    kind: Kind,

    /// 标签 → (偏移, 长度)。
    sections: HashMap<[u8; 4], (usize, usize)>,

    /// 第一节解析出来的元数据。
    metadata: Metadata,
}

impl Container {
    /// 打开并校验：魔数、版本、种类、分节表边界、`META` 可解析。
    pub fn open(path: &Path, expected: Kind) -> Result<Self, FormatError> {
        let file = File::open(path)?;
        // SAFETY: 数据文件只整体替换（写临时文件再改名），从不就地修改；映射期间内容不变。
        let map = unsafe { Mmap::map(&file)? };
        let _ = map.advise(memmap2::Advice::WillNeed);
        Self::from_map(Arc::new(map), expected)
    }

    /// 文件开头是不是 `.qj` 魔数（调用方据此在 TSV 与 `.qj` 之间选加载路径）。
    pub fn is_qj(path: &Path) -> bool {
        use std::io::Read;
        let mut magic = [0u8; MAGIC.len()];
        File::open(path)
            .and_then(|mut f| f.read_exact(&mut magic))
            .is_ok()
            && magic == MAGIC
    }

    fn from_map(map: Arc<Mmap>, expected: Kind) -> Result<Self, FormatError> {
        let header = Header::ref_from_prefix(&map[..])
            .map(|(h, _)| *h)
            .map_err(|_| FormatError::BadMagic)?;
        if header.magic != MAGIC {
            return Err(FormatError::BadMagic);
        }
        if header.version != FORMAT_VERSION {
            return Err(FormatError::UnsupportedVersion(header.version));
        }
        let kind = Kind::from_code(header.kind).ok_or(FormatError::WrongKind {
            expected,
            found: header.kind,
        })?;
        if kind != expected {
            return Err(FormatError::WrongKind {
                expected,
                found: header.kind,
            });
        }
        let count = header.section_count as usize;
        let table_bytes = map
            .get(Header::SIZE..Header::SIZE + count * SectionEntry::SIZE)
            .ok_or(FormatError::Truncated)?;
        let entries =
            <[SectionEntry]>::ref_from_bytes(table_bytes).map_err(|_| FormatError::Truncated)?;
        let mut sections = HashMap::with_capacity(count);
        for entry in entries {
            let offset = usize::try_from(entry.offset).map_err(|_| FormatError::Truncated)?;
            let length = usize::try_from(entry.length).map_err(|_| FormatError::Truncated)?;
            if offset % SECTION_ALIGN != 0
                || offset.checked_add(length).is_none_or(|end| end > map.len())
            {
                return Err(FormatError::Truncated);
            }
            sections.insert(entry.tag, (offset, length));
        }
        let &(offset, length) = sections
            .get(&META_TAG)
            .ok_or(FormatError::MissingSection(META_TAG))?;
        let metadata = std::str::from_utf8(&map[offset..offset + length])
            .map_err(|_| FormatError::Malformed {
                tag: META_TAG,
                reason: "metadata is not UTF-8",
            })
            .and_then(|text| Metadata::from_toml(text).map_err(FormatError::from))?;
        Ok(Self {
            map,
            kind,
            sections,
            metadata,
        })
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// 原始字节。
    pub fn bytes(&self, tag: [u8; 4]) -> Result<&[u8], FormatError> {
        let &(offset, length) = self
            .sections
            .get(&tag)
            .ok_or(FormatError::MissingSection(tag))?;
        Ok(&self.map[offset..offset + length])
    }

    /// 定长结构体数组分节。
    pub fn table<T: FromBytes + Immutable + KnownLayout>(
        &self,
        tag: [u8; 4],
    ) -> Result<Table<T>, FormatError> {
        let &(offset, length) = self
            .sections
            .get(&tag)
            .ok_or(FormatError::MissingSection(tag))?;
        Table::mapped(Arc::clone(&self.map), offset, length).ok_or(FormatError::Malformed {
            tag,
            reason: "length is not a multiple of the element size or misaligned",
        })
    }

    /// UTF-8 文本分节。
    pub fn text(&self, tag: [u8; 4]) -> Result<Text, FormatError> {
        let &(offset, length) = self
            .sections
            .get(&tag)
            .ok_or(FormatError::MissingSection(tag))?;
        Text::mapped(Arc::clone(&self.map), offset, length).ok_or(FormatError::Malformed {
            tag,
            reason: "text is not valid UTF-8",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Writer;

    fn sample(name: &str) -> (tempfile_path::TempPath, Metadata) {
        let metadata = Metadata {
            name: "测试".to_owned(),
            license: "MIT".to_owned(),
            entries: 3,
            ..Metadata::default()
        };
        let path = tempfile_path::TempPath::new(name);
        let numbers: [u32; 3] = [1, 2, 3];
        Writer::new(Kind::Dictionary, &metadata)
            .unwrap()
            .section(*b"NUMS", zerocopy::IntoBytes::as_bytes(&numbers[..]))
            .section(*b"TEXT", "你好 world".as_bytes())
            .write_to(&path.0)
            .unwrap();
        (path, metadata)
    }

    #[test]
    fn round_trips_sections_and_metadata() {
        let (path, metadata) = sample("round-trip");
        assert!(Container::is_qj(&path.0));
        let container = Container::open(&path.0, Kind::Dictionary).unwrap();
        assert_eq!(container.metadata(), &metadata);
        let numbers: Table<u32> = container.table(*b"NUMS").unwrap();
        assert_eq!(&*numbers, &[1, 2, 3]);
        let text = container.text(*b"TEXT").unwrap();
        assert_eq!(&*text, "你好 world");
        assert!(matches!(
            container.bytes(*b"NOPE"),
            Err(FormatError::MissingSection(_))
        ));
        // 3 个 u32 当 u64 表读：长度不整除
        assert!(matches!(
            container.table::<u64>(*b"NUMS"),
            Err(FormatError::Malformed { .. })
        ));
    }

    #[test]
    fn rejects_wrong_kind_and_garbage() {
        let (path, _) = sample("wrong-kind");
        assert!(matches!(
            Container::open(&path.0, Kind::LanguageModel),
            Err(FormatError::WrongKind { .. })
        ));
        let garbage = tempfile_path::TempPath::new("garbage");
        std::fs::write(
            &garbage.0,
            b"not a qj file at all, definitely longer than a header",
        )
        .unwrap();
        assert!(!Container::is_qj(&garbage.0));
        assert!(matches!(
            Container::open(&garbage.0, Kind::Dictionary),
            Err(FormatError::BadMagic)
        ));
    }

    /// 测试用的临时文件路径，drop 时删除。
    mod tempfile_path {
        use std::path::PathBuf;

        pub struct TempPath(pub PathBuf);

        impl TempPath {
            pub fn new(name: &str) -> Self {
                let dir = std::env::temp_dir().join("qingjian-format-tests");
                std::fs::create_dir_all(&dir).unwrap();
                Self(dir.join(format!("{name}-{}.qj", std::process::id())))
            }
        }

        impl Drop for TempPath {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }
    }
}
