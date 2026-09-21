//! 注音解码结果表示。
use super::unit::Unit;
use crate::parser::{Segmentation, Syllable as ParserSyllable};

/// 一段注音鍵解碼的結果：能解的單元 + 解不動的尾巴。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Decoded {
    units: Vec<Unit>,

    tail: String,

    pinyin: String,
}

impl Decoded {
    pub fn new(units: Vec<Unit>, tail: String) -> Self {
        let mut pinyin = String::with_capacity(units.len() * 6);
        for unit in &units {
            if unit.pinyin == "'" {
                continue;
            }
            if !pinyin.is_empty() {
                pinyin.push('\'');
            }
            pinyin.push_str(&unit.pinyin);
        }
        Self {
            units,
            tail,
            pinyin,
        }
    }

    pub fn units(&self) -> &[Unit] {
        &self.units
    }

    pub fn pinyin(&self) -> &str {
        &self.pinyin
    }

    pub fn tail(&self) -> &str {
        &self.tail
    }

    pub fn is_complete(&self) -> bool {
        self.tail.is_empty() && self.units.iter().all(|u| u.complete || u.pinyin == "'")
    }

    pub fn segmentation(&self) -> Option<Segmentation> {
        let syllables: Vec<ParserSyllable> = self
            .units
            .iter()
            .filter(|u| u.pinyin != "'")
            .map(|u| {
                if u.complete {
                    ParserSyllable::complete(&u.pinyin)
                } else {
                    ParserSyllable::partial(&u.pinyin)
                }
            })
            .collect();
        (!syllables.is_empty()).then_some(Segmentation { syllables })
    }

    pub fn marked(&self) -> String {
        let mut s = String::new();
        for unit in &self.units {
            s.push_str(&unit.display);
        }
        s.push_str(&self.tail);
        s
    }

    pub fn keys_for(&self, pinyin_len: usize) -> usize {
        let mut keys = 0;
        let mut position = 0;
        let mut first = true;
        let mut pending_separators = 0;
        for unit in &self.units {
            if unit.pinyin == "'" {
                pending_separators += unit.keys.len();
                continue;
            }
            let start = if first { 0 } else { position + 1 };
            let end = start + unit.pinyin.len();
            if pinyin_len < end {
                break;
            }
            keys += pending_separators + unit.keys.len();
            pending_separators = 0;
            position = end;
            first = false;
        }
        if keys > 0 {
            keys += pending_separators;
        }
        keys
    }
}
