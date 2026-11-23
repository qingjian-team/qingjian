//! [`Record`]：线上传的一条内容，及只带 id 与时刻的 [`Stamp`]。id 由内容算出，各端各自复制的同样内容自然是同一条。

mod kind;
mod stamp;

use sha2::{Digest, Sha256};

use crate::peers::hex;

pub use kind::RecordKind;
pub use stamp::Stamp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub stamp: Stamp,

    pub kind: RecordKind,

    /// 文字原样；图片为空。
    pub text: String,

    /// 图片文件内容；文字和删除标记为空。
    pub image: Vec<u8>,

    /// 图片扩展名（`png`）；文字为空。
    pub extension: String,
}

impl Record {
    pub fn text(text: String, at: i64) -> Self {
        Self {
            stamp: Stamp {
                id: content_id(RecordKind::Text, text.as_bytes()),
                at,
                deleted: false,
                forgotten: false,
            },
            kind: RecordKind::Text,
            text,
            image: Vec::new(),
            extension: String::new(),
        }
    }

    pub fn image(image: Vec<u8>, extension: String, at: i64) -> Self {
        Self {
            stamp: Stamp {
                id: content_id(RecordKind::Image, &image),
                at,
                deleted: false,
                forgotten: false,
            },
            kind: RecordKind::Image,
            text: String::new(),
            image,
            extension,
        }
    }

    /// 删除标记：只留 id 和时刻，内容不再传。
    pub fn tombstone(stamp: Stamp, kind: RecordKind) -> Self {
        Self {
            stamp: Stamp {
                deleted: true,
                ..stamp
            },
            kind,
            text: String::new(),
            image: Vec::new(),
            extension: String::new(),
        }
    }

    /// 内容和 id 对得上，或者是删除标记。收到别人的内容时要查，id 是合并的依据。
    pub fn is_consistent(&self) -> bool {
        if self.stamp.deleted {
            return self.text.is_empty() && self.image.is_empty();
        }
        let content = match self.kind {
            RecordKind::Text => self.text.as_bytes(),
            RecordKind::Image => self.image.as_slice(),
        };
        self.stamp.id == content_id(self.kind, content)
    }
}

/// 内容的 id。
pub fn content_id(kind: RecordKind, content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(match kind {
        RecordKind::Text => b"text\0",
        RecordKind::Image => b"imag\0",
    });
    hasher.update(content);
    hex(&hasher.finalize()[..16])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_stable_and_grow_with_the_content() {
        assert_eq!(
            content_id(RecordKind::Text, "你好".as_bytes()),
            content_id(RecordKind::Text, "你好".as_bytes())
        );
        assert_ne!(
            content_id(RecordKind::Text, "你好".as_bytes()),
            content_id(RecordKind::Text, "你好！".as_bytes())
        );
        assert_eq!(content_id(RecordKind::Text, b"x").len(), 32);
    }

    #[test]
    fn the_kind_is_part_of_the_id() {
        assert_ne!(
            content_id(RecordKind::Text, b"same"),
            content_id(RecordKind::Image, b"same")
        );
    }

    #[test]
    fn a_text_record_is_consistent_with_itself() {
        let record = Record::text("一段文字".to_owned(), 7);
        assert!(record.is_consistent());
        assert_eq!(record.kind, RecordKind::Text);
        assert!(record.image.is_empty());
        assert!(record.extension.is_empty());
        assert_eq!(record.stamp.at, 7);
        assert!(!record.stamp.deleted);
    }

    #[test]
    fn edited_text_breaks_the_consistency_check() {
        let mut record = Record::text("原文".to_owned(), 1);
        record.text = "改过".to_owned();
        assert!(!record.is_consistent());
    }

    #[test]
    fn images_are_checked_by_their_bytes() {
        let record = Record::image(vec![1, 2, 3], "png".to_owned(), 2);
        assert!(record.is_consistent());
        assert_eq!(record.kind, RecordKind::Image);
        assert!(record.text.is_empty());
        assert_eq!(record.extension, "png");
    }

    #[test]
    fn a_tombstone_carries_only_the_stamp() {
        let stamp = Record::text("要删的".to_owned(), 3).stamp;
        let id = stamp.id.clone();
        let tombstone = Record::tombstone(stamp, RecordKind::Text);
        assert!(tombstone.stamp.deleted);
        assert_eq!(tombstone.stamp.id, id);
        assert!(tombstone.text.is_empty());
        assert!(tombstone.image.is_empty());
        assert!(tombstone.is_consistent());
    }

    #[test]
    fn a_tombstone_with_content_is_inconsistent() {
        let mut tombstone =
            Record::tombstone(Record::text("x".to_owned(), 1).stamp, RecordKind::Text);
        tombstone.text = "x".to_owned();
        assert!(!tombstone.is_consistent());
    }
}
