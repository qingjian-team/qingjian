use std::path::Path;

use qingjian_format::{Container, FormatError, Kind, Metadata, Table, Text, Writer};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::error::DictionaryError;
use crate::matching::Match;
use crate::pattern::canonical_syllable;
use crate::tones::{self, SPILLED, Spill, Tones, split_tone};

use self::readings::Readings;

mod export;
mod lookup;
mod readings;

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
struct Slot {
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
struct KeyIndex {
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
struct Row {
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
    pub fn parse(source: &str) -> Result<Self, DictionaryError> {
        // 记事本存的「UTF-8 带 BOM」：`trim` 不去 U+FEFF，第一个词会带着它上屏
        let source = source.strip_prefix('\u{feff}').unwrap_or(source);
        let mut texts = String::new();
        let mut keys = String::new();
        let mut tones: Vec<u8> = Vec::new();
        let mut rows: Vec<Row> = Vec::new();
        for (index, raw) in source.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let number = index + 1;
            let mut fields = line.split('\t');
            let text = fields
                .next()
                .filter(|s| !s.is_empty())
                .ok_or(DictionaryError::Line {
                    line: number,
                    reason: "missing word",
                })?;
            let pinyin = fields
                .next()
                .filter(|s| !s.is_empty())
                .ok_or(DictionaryError::Line {
                    line: number,
                    reason: "missing pinyin",
                })?;
            let frequency = fields
                .next()
                .map(|s| s.trim().parse::<u32>())
                .transpose()
                .map_err(|_| DictionaryError::Line {
                    line: number,
                    reason: "frequency is not a non-negative integer",
                })?
                .unwrap_or(1);
            let text_len = u16::try_from(text.len()).map_err(|_| DictionaryError::Line {
                line: number,
                reason: "word too long",
            })?;
            let key_len = u16::try_from(pinyin.len()).map_err(|_| DictionaryError::Line {
                line: number,
                reason: "pinyin too long",
            })?;
            rows.push(Row {
                key_start: keys.len() as u32,
                key_len,
                tone_start: tones.len() as u32,
                slot: Slot {
                    text_start: texts.len() as u32,
                    frequency,
                    text_len,
                    tones: 0,
                },
            });
            texts.push_str(text);
            // 规范化：音节之间恰好一个空格，数字声调拿出来另存（`ni3 hao3` 与 `ni hao` 是同一个键）
            let mut first = true;
            for syllable in pinyin.split_whitespace() {
                if !first {
                    keys.push(' ');
                }
                let (bare, tone) = split_tone(syllable);
                keys.push_str(canonical_syllable(bare));
                tones.push(tone);
                first = false;
            }
            let last = rows.last_mut().unwrap();
            last.key_len = (keys.len() as u32 - last.key_start) as u16;
        }
        Ok(Self::assemble(texts, keys, rows, &tones))
    }

    /// 按键排序、同键归并。`tones` 是每行从 `tone_start` 起、按键的音节数逐个排的声调。
    fn assemble(texts: String, keys: String, mut rows: Vec<Row>, tones: &[u8]) -> Self {
        let key_of = |row: &Row| {
            &keys[row.key_start as usize..row.key_start as usize + row.key_len as usize]
        };
        rows.sort_by(|a, b| {
            key_of(a)
                .cmp(key_of(b))
                .then_with(|| b.slot.frequency.cmp(&a.slot.frequency))
        });
        let mut index: Vec<KeyIndex> = Vec::new();
        let mut slots: Vec<Slot> = Vec::with_capacity(rows.len());
        let mut spills: Vec<Spill> = Vec::new();
        let mut spilled = String::new();
        for row in &rows {
            match index.last_mut() {
                Some(last)
                    if key_of(row)
                        == &keys[last.key_start as usize
                            ..last.key_start as usize + last.key_len as usize] =>
                {
                    last.slot_count += 1;
                }
                _ => index.push(KeyIndex {
                    key_start: row.key_start,
                    first_slot: slots.len() as u32,
                    slot_count: 1,
                    key_len: row.key_len,
                    reserved: 0,
                }),
            }
            let start = row.tone_start as usize;
            let count = key_of(row).split(' ').count();
            let row_tones = &tones[start..start + count];
            let packed = tones::pack(row_tones);
            if packed == SPILLED {
                spills.push(Spill {
                    slot: slots.len() as u32,
                    start: spilled.len() as u32,
                    len: count as u16,
                    reserved: 0,
                });
                spilled.extend(row_tones.iter().map(|&t| char::from(b'0' + t)));
            }
            slots.push(Slot {
                tones: packed,
                ..row.slot
            });
        }
        let total_frequency = slots.iter().map(|s| u64::from(s.frequency)).sum();
        tracing::debug!(entries = slots.len(), keys = index.len(), "词库加载完成");
        Self {
            texts: Text::Owned(texts),
            keys: Text::Owned(keys),
            index: Table::Owned(index),
            slots: Table::Owned(slots),
            spills: Table::Owned(spills),
            spilled: Text::Owned(spilled),
            total_frequency,
            metadata: None,
            secondary: false,
            chars_only: false,
            readings: Readings::default(),
        }
        .with_readings()
    }

    /// 按文件内容选加载方式：`.qj` 容器直接映射，否则当 TSV 解析。
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, DictionaryError> {
        let path = path.as_ref();
        if Container::is_qj(path) {
            Self::open_qj(path)
        } else {
            Self::parse(&std::fs::read_to_string(path)?)
        }
    }

    /// 打开 `.qj` 词库：映射四个分节，逐条校验偏移落在 arena 内且在字符边界上（坏文件报错而不是查询时 panic）。
    pub fn open_qj(path: &Path) -> Result<Self, DictionaryError> {
        let container = Container::open(path, Kind::Dictionary)?;
        let texts = container.text(TEXT_TAG)?;
        let keys = container.text(KEYS_TAG)?;
        let index: Table<KeyIndex> = container.table(INDEX_TAG)?;
        let slots: Table<Slot> = container.table(SLOTS_TAG)?;
        let (spills, spilled) = match container.table::<Spill>(SPILL_TAG) {
            Ok(spills) => (spills, container.text(TONES_TAG)?),
            Err(FormatError::MissingSection(_)) => (Table::default(), Text::default()),
            Err(error) => return Err(error.into()),
        };
        let mut previous = None;
        for spill in spills.iter() {
            let start = spill.start as usize;
            if spilled.get(start..start + usize::from(spill.len)).is_none()
                || spill.slot as usize >= slots.len()
                || previous.is_some_and(|slot| slot >= spill.slot)
            {
                return Err(DictionaryError::Corrupt("tone spill out of order or range"));
            }
            previous = Some(spill.slot);
        }
        let mut total_frequency = 0u64;
        for slot in slots.iter() {
            let start = slot.text_start as usize;
            if texts
                .get(start..start + usize::from(slot.text_len))
                .is_none()
            {
                return Err(DictionaryError::Corrupt(
                    "slot points outside the text arena",
                ));
            }
            total_frequency += u64::from(slot.frequency);
        }
        for entry in index.iter() {
            let start = entry.key_start as usize;
            if keys
                .get(start..start + usize::from(entry.key_len))
                .is_none()
            {
                return Err(DictionaryError::Corrupt("key points outside the key arena"));
            }
            let first = entry.first_slot as usize;
            if first + entry.slot_count as usize > slots.len() {
                return Err(DictionaryError::Corrupt(
                    "key points outside the slot table",
                ));
            }
        }
        let has_legacy_keys = index.iter().any(|entry| {
            let start = entry.key_start as usize;
            keys[start..start + usize::from(entry.key_len)]
                .split(' ')
                .any(|syllable| canonical_syllable(syllable) != syllable)
        });
        if has_legacy_keys {
            let metadata = container.metadata().clone();
            let mut owned_keys = String::with_capacity(keys.len());
            let mut owned_tones = Vec::new();
            let mut rows = Vec::with_capacity(slots.len());
            for entry in index.iter() {
                let start = entry.key_start as usize;
                let key = &keys[start..start + usize::from(entry.key_len)];
                let key_start = owned_keys.len() as u32;
                for (position, syllable) in key.split(' ').enumerate() {
                    if position > 0 {
                        owned_keys.push(' ');
                    }
                    owned_keys.push_str(canonical_syllable(syllable));
                }
                let key_len = (owned_keys.len() as u32 - key_start) as u16;
                let count = key.split(' ').count();
                let first = entry.first_slot as usize;
                for (i, slot) in slots[first..first + entry.slot_count as usize]
                    .iter()
                    .enumerate()
                {
                    let tone_start = owned_tones.len() as u32;
                    let slot_tones = slot_tones(&spills, &spilled, first + i, slot);
                    owned_tones.extend((0..count).map(|i| slot_tones.get(i)));
                    rows.push(Row {
                        key_start,
                        key_len,
                        tone_start,
                        slot: *slot,
                    });
                }
            }
            let mut dictionary = Self::assemble(texts.to_string(), owned_keys, rows, &owned_tones);
            dictionary.metadata = Some(metadata);
            tracing::debug!(
                entries = dictionary.slots.len(),
                keys = dictionary.index.len(),
                "旧词库键已规范化"
            );
            return Ok(dictionary);
        }
        tracing::debug!(
            entries = slots.len(),
            keys = index.len(),
            name = %container.metadata().name,
            "词库已映射"
        );
        Ok(Self {
            texts,
            keys,
            index,
            slots,
            spills,
            spilled,
            total_frequency,
            metadata: Some(container.metadata().clone()),
            secondary: false,
            chars_only: false,
            readings: Readings::default(),
        }
        .with_readings())
    }

    fn with_readings(mut self) -> Self {
        self.readings = Readings::build(&self);
        self
    }

    /// 写成 `.qj`：内存里的四段原样落盘。`metadata.entries` 会填成词条数。
    pub fn write_qj(&self, path: &Path, metadata: &Metadata) -> Result<(), DictionaryError> {
        let metadata = Metadata {
            entries: self.slots.len() as u64,
            ..metadata.clone()
        };
        let mut writer = Writer::new(Kind::Dictionary, &metadata)?
            .section(TEXT_TAG, self.texts.as_bytes())
            .section(KEYS_TAG, self.keys.as_bytes())
            .section(INDEX_TAG, self.index.as_bytes())
            .section(SLOTS_TAG, self.slots.as_bytes());
        if !self.spills.is_empty() {
            writer = writer
                .section(SPILL_TAG, self.spills.as_bytes())
                .section(TONES_TAG, self.spilled.as_bytes());
        }
        writer.write_to(path)?;
        Ok(())
    }

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

    /// `.qj` 里的来历；TSV 解析的返回 `None`。
    pub fn metadata(&self) -> Option<&Metadata> {
        self.metadata.as_ref()
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// 全部词频之和。
    pub fn total_frequency(&self) -> u64 {
        self.total_frequency
    }

    /// 全部词目，按拼音键的字节序、同一个键下按词频降序。给反查（汉字 → 读音）建索引用。
    pub fn entries(&self) -> impl Iterator<Item = Match<'_>> + '_ {
        self.index.iter().flat_map(move |entry| {
            let key = self.key(entry);
            let range =
                entry.first_slot as usize..entry.first_slot as usize + entry.slot_count as usize;
            range.map(move |position| {
                let slot = &self.slots[position];
                Match {
                    text: self.text(slot),
                    pinyin: key,
                    frequency: slot.frequency,
                    reading_total: self.reading_total(key, self.text(slot)),
                    tones: self.tones(position, slot),
                    secondary: self.secondary,
                    exact: true,
                }
            })
        })
    }

    fn key(&self, entry: &KeyIndex) -> &str {
        &self.keys[entry.key_start as usize..entry.key_start as usize + entry.key_len as usize]
    }

    fn text(&self, slot: &Slot) -> &str {
        &self.texts[slot.text_start as usize..slot.text_start as usize + slot.text_len as usize]
    }

    /// 单音节的键才可能是多音字，多音节的不查表（单字母简拼一查几万条）。
    fn reading_total(&self, key: &str, text: &str) -> u32 {
        if key.contains(' ') {
            0
        } else {
            self.readings.total(text)
        }
    }

    fn tones(&self, position: usize, slot: &Slot) -> Tones<'_> {
        slot_tones(&self.spills, &self.spilled, position, slot)
    }
}

/// 第 `position` 条词目的声调：压在词目里的直接解开，溢出的到溢出表里二分找。
fn slot_tones<'a>(spills: &[Spill], spilled: &'a str, position: usize, slot: &Slot) -> Tones<'a> {
    if slot.tones & SPILLED == 0 {
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

#[cfg(test)]
mod tests;
