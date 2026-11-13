use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

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
    pub fn write_to(self, path: &Path) -> Result<(), FormatError> {
        let temp = path.with_extension("qj.tmp");
        {
            let file = File::create(&temp)?;
            let mut out = BufWriter::new(file);
            self.write(&mut out)?;
            out.flush()?;
            out.get_ref().sync_all()?;
        }
        std::fs::rename(&temp, path)?;
        Ok(())
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
