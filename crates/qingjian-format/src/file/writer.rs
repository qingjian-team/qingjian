use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use zerocopy::IntoBytes;

use crate::{
    FORMAT_VERSION, FormatError, Header, Kind, MAGIC, META_TAG, Metadata, SECTION_ALIGN,
    SectionEntry,
};

/// 攒分节、写文件。分节按加入顺序落盘，`META` 自动排第一。
pub struct Writer {
    /// 数据种类。
    kind: Kind,

    /// (标签, 正文)。
    sections: Vec<([u8; 4], Vec<u8>)>,
}

impl Writer {
    pub fn new(kind: Kind, metadata: &Metadata) -> Result<Self, FormatError> {
        Ok(Self {
            kind,
            sections: vec![(META_TAG, metadata.to_toml()?.into_bytes())],
        })
    }

    /// 加一节。标签在同一文件里不能重复。
    pub fn section(mut self, tag: [u8; 4], bytes: &[u8]) -> Self {
        debug_assert!(
            !self.sections.iter().any(|(t, _)| *t == tag),
            "duplicate section tag"
        );
        self.sections.push((tag, bytes.to_vec()));
        self
    }

    /// 写到 `path`：先写同目录临时文件再改名，读的一方永远看不到半个文件。
    ///
    /// 临时名带进程号与进程内序号：两个线程同时写同一个目标时不会互相截断；
    /// `create_new` 保证预先放好的同名符号链接不会被跟进去写别人的文件。
    pub fn write_to(self, path: &Path) -> Result<(), FormatError> {
        let temp = temporary_path(path);
        let outcome = (|| {
            let file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)?;
            let mut out = BufWriter::new(file);
            self.write(&mut out)?;
            out.flush()?;
            out.get_ref().sync_all()?;
            std::fs::rename(&temp, path)?;
            Ok::<(), FormatError>(())
        })();
        if outcome.is_err() {
            let _ = std::fs::remove_file(&temp);
        }
        outcome
    }

    /// 写到任意输出。
    pub fn write(self, out: &mut impl Write) -> Result<(), FormatError> {
        let count = self.sections.len();
        let header = Header {
            magic: MAGIC,
            version: FORMAT_VERSION,
            kind: self.kind.code(),
            section_count: u32::try_from(count).map_err(|_| FormatError::Truncated)?,
            reserved: [0; 16],
        };
        let table_end = Header::SIZE + count * SectionEntry::SIZE;
        let mut offset = align_up(table_end);
        let mut entries = Vec::with_capacity(count);
        for (tag, bytes) in &self.sections {
            entries.push(SectionEntry {
                tag: *tag,
                reserved: 0,
                offset: offset as u64,
                length: u64::try_from(bytes.len())
                    .map_err(|_| FormatError::SectionTooLarge(*tag))?,
            });
            offset = align_up(offset + bytes.len());
        }
        out.write_all(header.as_bytes())?;
        for entry in &entries {
            out.write_all(entry.as_bytes())?;
        }
        write_padding(out, table_end)?;
        let mut position = align_up(table_end);
        for (_, bytes) in &self.sections {
            out.write_all(bytes)?;
            position += bytes.len();
            write_padding(out, position)?;
            position = align_up(position);
        }
        Ok(())
    }
}

/// 向上对齐到 [`SECTION_ALIGN`]。
pub(crate) fn align_up(offset: usize) -> usize {
    offset.div_ceil(SECTION_ALIGN) * SECTION_ALIGN
}

fn temporary_path(path: &Path) -> PathBuf {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    path.with_extension(format!("qj.tmp-{}-{sequence}", std::process::id()))
}

fn write_padding(out: &mut impl Write, position: usize) -> Result<(), FormatError> {
    let padding = align_up(position) - position;
    if padding > 0 {
        out.write_all(&[0u8; SECTION_ALIGN][..padding])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_writes_leave_one_whole_file() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("qingjian-writer-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("concurrent.qj");
        let handles: Vec<_> = (0..8)
            .map(|index| {
                let path = path.clone();
                std::thread::spawn(move || {
                    let marker = format!("version-{index}-{}", "x".repeat(256));
                    Writer::new(Kind::Dictionary, &Metadata::default())
                        .unwrap()
                        .section(*b"MARK", marker.as_bytes())
                        .write_to(&path)
                        .unwrap();
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
        let container = crate::Container::open(&path, Kind::Dictionary).unwrap();
        let marker = container.bytes(*b"MARK").unwrap();
        let marker = std::str::from_utf8(marker).unwrap();
        assert!(marker.starts_with("version-"), "混合内容：{marker}");
        assert_eq!(marker.len(), "version-0-".len() + 256, "被截断：{marker}");
        assert!(marker.ends_with("xxxx"), "混合内容：{marker}");
        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".qj.tmp-"))
            .collect();
        assert!(leftovers.is_empty(), "临时文件没清：{leftovers:?}");
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn align_up_rounds_to_section_align() {
        assert_eq!(align_up(0), 0);
        assert_eq!(align_up(1), SECTION_ALIGN);
        assert_eq!(align_up(SECTION_ALIGN), SECTION_ALIGN);
        assert_eq!(align_up(SECTION_ALIGN + 1), SECTION_ALIGN * 2);
    }

    #[test]
    fn section_offsets_are_aligned_and_padding_is_zero() {
        let mut buf = Vec::new();
        Writer::new(Kind::Dictionary, &Metadata::default())
            .unwrap()
            .section(*b"ABCD", b"123")
            .section(*b"EFGH", b"456789")
            .write(&mut buf)
            .unwrap();
        let u64_at = |b: &[u8]| u64::from_le_bytes(b.try_into().unwrap());
        let count = u32::from_le_bytes(buf[12..16].try_into().unwrap()) as usize;
        assert_eq!(count, 3);
        for k in 0..count {
            let base = Header::SIZE + k * SectionEntry::SIZE;
            let offset = u64_at(&buf[base + 8..base + 16]) as usize;
            let length = u64_at(&buf[base + 16..base + 24]) as usize;
            assert_eq!(offset % SECTION_ALIGN, 0, "section {k} offset");
            let end = offset + length;
            assert!(end <= buf.len(), "section {k} inside the written bytes");
            assert!(
                buf[end..align_up(end)].iter().all(|b| *b == 0),
                "section {k} padding"
            );
            if k + 1 < count {
                let next =
                    u64_at(&buf[base + SectionEntry::SIZE + 8..base + SectionEntry::SIZE + 16])
                        as usize;
                assert_eq!(next, align_up(end), "section {} follows the padding", k + 1);
            }
        }
    }

    #[test]
    #[should_panic(expected = "duplicate section tag")]
    fn duplicate_tags_are_rejected() {
        let _ = Writer::new(Kind::Dictionary, &Metadata::default())
            .unwrap()
            .section(*b"ABCD", b"1")
            .section(*b"ABCD", b"2");
    }
}
