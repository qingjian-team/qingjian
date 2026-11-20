//! 内存词库：arena 存放词文本与拼音键，键排序后二分收窄查询。
//!
//! 分域：[`open`] 负责 TSV 解析 / `.qj` 打开与落盘，[`query`] 是对外查词入口，
//! [`narrow`] 是区间收窄引擎；本文件只留数据结构、内部取值访问器与维护标记。

use qingjian_format::{Metadata, Table, Text};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::matching::Match;
use crate::tones::{Spill, Tones};

use self::readings::Readings;

mod export;
mod narrow;
mod open;
mod query;
mod readings;

#[cfg(test)]
mod tests;

/// 收窄到多小的区间就改成逐键比对。
const LINEAR_SCAN_LIMIT: usize = 48;

/// `.qj` 里的分节：词文本 arena、拼音键 arena、键索引、词目。
const TEXT_TAG: [u8; 4] = *b"TEXT";
const KEYS_TAG: [u8; 4] = *b"KEYS";
const INDEX_TAG: [u8; 4] = *b"INDX";
const SLOTS_TAG: [u8; 4] = *b"SLOT";

/// 五个音节以上的词的声调：溢出表与声调 arena。没有长词带声调的词库（以及带声调之前的旧文件）没有这两节。
const SPILL_TAG: [u8; 4] = *b"TSPL";
const TONES_TAG: [u8; 4] = *b"TONE";

/// 一条词目：词文本在 arena 里的位置、词频与声调。12 字节、无填充，原样落盘。
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub(super) struct Slot {
    /// 在 `Dictionary::texts` 里的字节偏移。
    text_start: u32,

    /// 静态词频。
    frequency: u32,

    /// 字节长度。
    text_len: u16,

    /// 逐音节声调，见 [`tones`] 模块；带声调之前的文件里全零，就是不知道。
    tones: u16,
}

/// 一个拼音键（如 `kai fa`）及其下的词目范围。16 字节、无填充，原样落盘。
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub(super) struct KeyIndex {
    /// 在 `Dictionary::keys` 里的字节偏移。
    key_start: u32,

    /// 在 `Dictionary::slots` 里的起始下标。
    first_slot: u32,

    /// 词目个数（同音词）。
    slot_count: u32,

    /// 字节长度。
    key_len: u16,

    /// 对齐用，全零。
    reserved: u16,
}

/// 解析阶段的临时行，排序后才归并成 `KeyIndex` + `Slot`。声调在另一个缓冲里，`slot.tones` 到归并时才填。
#[derive(Debug, Clone, Copy)]
pub(super) struct Row {
    key_start: u32,
    key_len: u16,
    tone_start: u32,
    slot: Slot,
}

/// 内存词库。从 TSV 解析时四段数据是自己的，从 `.qj` 打开时是映射文件里的，查询代码不区分。
#[derive(Debug, Default)]
pub struct Dictionary {
    /// 所有词文本首尾相接。
    texts: Text,

    /// 所有拼音键首尾相接（同音词共用一个键）。
    keys: Text,

    /// 按键的字节序升序排列。
    index: Table<KeyIndex>,

    /// 与 `index` 同序；同一个键下按词频降序。
    slots: Table<Slot>,

    /// 长词声调的溢出表，按词目下标升序。
    spills: Table<Spill>,

    /// 溢出表指向的声调，每个音节一个 `0`–`5`。
    spilled: Text,

    /// 全部词频之和，整句转换把词频归一化成概率时用。
    total_frequency: u64,

    /// `.qj` 里的来历（名称、许可证、署名）；TSV 解析的没有。
    metadata: Option<Metadata>,

    /// 补充词库（通用大词库）：命中带上 [`Match::secondary`]，排序与整句里低主词库一档。不落盘，由装配方标。
    secondary: bool,

    /// 只出单字：多音节的键查词时跳过。用户在设置里关掉基础词库时由装配方标，单字仍要靠它出。不落盘。
    chars_only: bool,

    /// 多音字全部读音的词频之和，加载时从单音节的键算出来，不落盘。
    readings: Readings,
}

impl Dictionary {
    /// 标成补充词库，见 [`Match::secondary`]。
    pub fn set_secondary(&mut self, secondary: bool) {
        self.secondary = secondary;
    }

    pub fn is_secondary(&self) -> bool {
        self.secondary
    }

    /// 只出单字，见 `chars_only` 字段。
    pub fn set_chars_only(&mut self, chars_only: bool) {
        self.chars_only = chars_only;
    }

    pub fn is_chars_only(&self) -> bool {
        self.chars_only
    }

    pub(super) fn key(&self, entry: &KeyIndex) -> &str {
        &self.keys[entry.key_start as usize..entry.key_start as usize + entry.key_len as usize]
    }

    pub(super) fn text(&self, slot: &Slot) -> &str {
        &self.texts[slot.text_start as usize..slot.text_start as usize + slot.text_len as usize]
    }

    /// 把一个键下的全部词目追加为命中。
    pub(super) fn push_matches<'a>(
        &'a self,
        entry: &KeyIndex,
        exact: bool,
        out: &mut Vec<Match<'a>>,
    ) {
        let key = self.key(entry);
        if self.chars_only && key.contains(' ') {
            return;
        }
        let range =
            entry.first_slot as usize..entry.first_slot as usize + entry.slot_count as usize;
        for position in range {
            let slot = &self.slots[position];
            out.push(Match {
                text: self.text(slot),
                pinyin: key,
                frequency: slot.frequency,
                reading_total: self.reading_total(key, self.text(slot)),
                tones: self.tones(position, slot),
                secondary: self.secondary,
                exact,
            });
        }
    }

    /// 单音节的键才可能是多音字，多音节的不查表（单字母简拼一查几万条）。
    pub(super) fn reading_total(&self, key: &str, text: &str) -> u32 {
        if key.contains(' ') {
            0
        } else {
            self.readings.total(text)
        }
    }

    pub(super) fn tones(&self, position: usize, slot: &Slot) -> Tones<'_> {
        slot_tones(&self.spills, &self.spilled, position, slot)
    }
}

/// 第 `position` 条词目的声调：压在词目里的直接解开，溢出的到溢出表里二分找。
pub(super) fn slot_tones<'a>(
    spills: &[Spill],
    spilled: &'a str,
    position: usize,
    slot: &Slot,
) -> Tones<'a> {
    if slot.tones & crate::tones::SPILLED == 0 {
        return Tones::packed(slot.tones);
    }
    spills
        .binary_search_by_key(&(position as u32), |spill| spill.slot)
        .ok()
        .and_then(|i| {
            let spill = &spills[i];
            let start = spill.start as usize;
            spilled
                .as_bytes()
                .get(start..start + usize::from(spill.len))
        })
        .map_or_else(Tones::default, Tones::spilled)
}
