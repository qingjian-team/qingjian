//! 词库的打开与加载：TSV 解析装配、`.qj` 映射打开、落盘写回。

use std::path::Path;

use qingjian_format::{Container, FormatError, Kind, Metadata, Table, Text, Writer};

use crate::error::DictionaryError;
use crate::pattern::canonical_syllable;
use crate::tones::{self, SPILLED, Spill};

use super::{
    Dictionary, INDEX_TAG, KEYS_TAG, KeyIndex, Readings, Row, SLOTS_TAG, SPILL_TAG, Slot, TEXT_TAG,
    TONES_TAG, slot_tones,
};

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
                let (bare, tone) = crate::tones::split_tone(syllable);
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
}
