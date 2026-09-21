//! 按输入来源映射注音光标；未知尾串与已解码音节分别累计。
use crate::zhuyin::{layout::map_key, syllable::ZhuyinSyllable};

pub(super) struct Cursor {
    input: usize,

    units_len: usize,

    tail_len: usize,

    unit_cursor: usize,

    tail_cursor: Option<usize>,
}

impl Cursor {
    pub(super) fn new(input: usize) -> Self {
        Self {
            input,
            units_len: 0,
            tail_len: 0,
            unit_cursor: 0,
            tail_cursor: None,
        }
    }

    pub(super) fn unit(&mut self, start: usize, keys: &str, display: &str) {
        if self.input > start && self.input <= start + keys.len() {
            let end = if self.input == start + keys.len() || keys == "'" {
                display.len()
            } else {
                let mut prefix = ZhuyinSyllable::new();
                for (offset, key) in keys.char_indices() {
                    if start + offset + key.len_utf8() > self.input {
                        break;
                    }
                    if let Some(component) = map_key(key) {
                        prefix.push(component, key);
                    }
                }
                // 前置轻声会被移到单元末尾。按逻辑符号数映射，不能把 ˙ 的
                // 两字节长度直接当作完整单元内的偏移（可能落在 ㄋ 的中间）。
                display
                    .char_indices()
                    .nth(prefix.display_string().chars().count())
                    .map_or(display.len(), |(offset, _)| offset)
            };
            self.unit_cursor = self.units_len + end;
            self.tail_cursor = None;
        }
        self.units_len += display.len();
    }

    pub(super) fn tail(&mut self, start: usize, character: char) {
        self.tail_len += character.len_utf8();
        if self.input > start && self.input <= start + character.len_utf8() {
            self.tail_cursor = Some(self.tail_len);
        }
    }

    pub(super) fn finish(self, input_len: usize) -> usize {
        if self.input == 0 {
            return 0;
        }
        if self.input >= input_len {
            return self.units_len + self.tail_len;
        }
        self.tail_cursor
            .map_or(self.unit_cursor, |tail| self.units_len + tail)
    }
}
