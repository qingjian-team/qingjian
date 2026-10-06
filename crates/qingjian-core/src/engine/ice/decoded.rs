//! 全拼的等长容错写法；原始按键和上屏消费位置保持一一对应。

use crate::engine::segment_longest_prefix;
use crate::parser::{self, Segmentation};

pub(in crate::engine) struct Decoded {
    pinyin: String,

    tail: String,

    segmentation: Segmentation,
}

impl Decoded {
    pub(in crate::engine) fn new(keys: &str) -> Option<Self> {
        if !keys.is_ascii() {
            return None;
        }
        let mut bytes = keys.as_bytes().to_vec();
        for i in 1..bytes.len() {
            if bytes[i] == b'v' && matches!(bytes[i - 1], b'j' | b'q' | b'x' | b'y') {
                bytes[i] = b'u';
            }
        }
        // zh/ch/sh 的相邻键换位只在换回后成为完整音节时接受。
        for i in 0..bytes.len().saturating_sub(2) {
            let pair = if bytes[i] == b'h' && matches!(bytes[i + 1], b'z' | b'c' | b's') {
                Some((i, i + 1))
            } else if matches!(bytes[i], b'z' | b'c' | b's')
                && matches!(bytes[i + 1], b'a' | b'e' | b'i' | b'u')
                && bytes[i + 2] == b'h'
            {
                Some((i + 1, i + 2))
            } else {
                None
            };
            if let Some((a, b)) = pair {
                bytes.swap(a, b);
                let rest = std::str::from_utf8(&bytes[i..]).ok()?;
                if !(3..=rest.len().min(6)).any(|n| parser::is_syllable(&rest[..n])) {
                    bytes.swap(a, b);
                }
            }
        }
        let normalized = String::from_utf8(bytes).ok()?;
        if normalized == keys {
            return None;
        }
        let (segmentations, tail) = segment_longest_prefix(&normalized).ok()?;
        let tail = tail.to_owned();
        let head_len = normalized.len() - tail.len();
        Some(Self {
            pinyin: normalized[..head_len].to_owned(),
            tail,
            segmentation: segmentations.into_iter().next()?,
        })
    }

    pub(in crate::engine) fn pinyin(&self) -> &str {
        &self.pinyin
    }

    pub(in crate::engine) fn tail(&self) -> &str {
        &self.tail
    }

    pub(in crate::engine) fn segmentation(&self) -> Option<Segmentation> {
        Some(self.segmentation.clone())
    }

    pub(in crate::engine) fn marked(&self) -> String {
        let head = self.segmentation.joined("'");
        if self.tail.is_empty() {
            head
        } else {
            format!("{head}'{}", self.tail)
        }
    }

    pub(in crate::engine) fn keys_for(&self, pinyin_len: usize) -> usize {
        pinyin_len.min(self.pinyin.len())
    }

    pub(in crate::engine) fn is_complete(&self) -> bool {
        self.tail.is_empty() && self.segmentation.incomplete_count() == 0
    }
}
