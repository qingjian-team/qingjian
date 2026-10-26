//! 一条本地整句候选的异步译文。

use qingjian_core::Translation;

pub(crate) struct SentenceTranslation {
    pub(crate) text: String,

    pub(crate) sequence: u64,

    pub(crate) translation: Option<Translation>,
}

impl SentenceTranslation {
    pub(crate) fn pending(text: String, sequence: u64) -> Self {
        Self {
            text,
            sequence,
            translation: None,
        }
    }
}
