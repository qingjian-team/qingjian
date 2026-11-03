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
