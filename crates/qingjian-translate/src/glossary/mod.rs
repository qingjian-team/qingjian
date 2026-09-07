//! 释义表：`词\t[词性. ]译文[|读音]\t…` 的 TSV，或打包好的 `.qj`（mmap 零拷贝，启动不用解析 20 多万行）。

mod entry_record;
mod sense_record;
mod storage;

use std::collections::HashMap;
use std::path::Path;

use qingjian_core::{Language, PartOfSpeech, Sense, Translation, Translator};
use qingjian_format::{Container, Kind, Metadata, Table, Writer, hash};

use crate::error::GlossaryError;
use entry_record::EntryRecord;
use sense_record::SenseRecord;
use storage::Storage;

/// `.qj` 里的分节：字符串 arena、词条表、释义表、哈希索引。
const TEXT_TAG: [u8; 4] = *b"TEXT";
const ENTRIES_TAG: [u8; 4] = *b"ENTR";
const SENSES_TAG: [u8; 4] = *b"SENS";
const HASH_TAG: [u8; 4] = *b"HASH";

/// 释义表，一个实例对应一种学习语言。
#[derive(Debug)]
pub struct Glossary {
    /// 本表的学习语言。
    language: Language,

    /// 词 → 译文。
    storage: Storage,
}

impl Glossary {
    pub fn empty(language: Language) -> Self {
        Self {
            language,
            storage: Storage::Owned(HashMap::new()),
        }
    }

    pub fn parse(language: Language, source: &str) -> Result<Self, GlossaryError> {
        let mut entries: HashMap<String, Translation> = HashMap::new();
        for (index, raw) in source.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let number = index + 1;
            let mut fields = line.split('\t').map(str::trim);
            let text =
                fields
                    .next()
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| GlossaryError::Line {
                        line: number,
                        reason: "missing word".into(),
                    })?;
            let senses: Vec<Sense> = fields
                .filter(|s| !s.is_empty())
                .take(Translation::MAX_SENSES)
                .map(parse_sense)
                .collect();
            if senses.is_empty() {
                return Err(GlossaryError::Line {
                    line: number,
                    reason: "missing senses".into(),
                });
            }
            entries.insert(text.to_owned(), Translation::new(language, senses));
        }
        tracing::debug!(
            language = language.code(),
            entries = entries.len(),
            "释义表加载完成"
        );
        Ok(Self {
            language,
            storage: Storage::Owned(entries),
        })
    }

    /// 按文件内容选加载方式：`.qj` 直接映射，否则当 TSV 解析。
    pub fn from_path(language: Language, path: impl AsRef<Path>) -> Result<Self, GlossaryError> {
        let path = path.as_ref();
        if Container::is_qj(path) {
            Self::open_qj(language, path)
        } else {
            Self::parse(language, &std::fs::read_to_string(path)?)
        }
    }

    /// 打开 `.qj` 释义表，逐条校验偏移落在 arena 内、在字符边界上。
    pub fn open_qj(language: Language, path: &Path) -> Result<Self, GlossaryError> {
        let container = Container::open(path, Kind::Glossary)?;
        let text = container.text(TEXT_TAG)?;
        let entries: Table<EntryRecord> = container.table(ENTRIES_TAG)?;
        let senses: Table<SenseRecord> = container.table(SENSES_TAG)?;
        let index: Table<u32> = container.table(HASH_TAG)?;
        let in_arena = |start: u32, len: u16| {
            text.get(start as usize..start as usize + usize::from(len))
                .is_some()
        };
        for entry in entries.iter() {
            if !in_arena(entry.word_start, entry.word_len)
                || entry.sense_start as usize + usize::from(entry.sense_count) > senses.len()
            {
                return Err(GlossaryError::Corrupt(
                    "entry points outside the arena or sense table",
                ));
            }
        }
        for sense in senses.iter() {
            if !in_arena(sense.text_start, sense.text_len)
                || !in_arena(sense.reading_start, sense.reading_len)
                || !in_arena(sense.pos_start, sense.pos_len)
            {
                return Err(GlossaryError::Corrupt("sense points outside the arena"));
            }
        }
        if !hash::is_valid(&index, entries.len()) {
            return Err(GlossaryError::Corrupt(
                "hash index does not match the entry table",
            ));
        }
        tracing::debug!(
            language = language.code(),
            entries = entries.len(),
            name = %container.metadata().name,
            "释义表已映射"
        );
        Ok(Self {
            language,
            storage: Storage::Mapped {
                text,
                entries,
                senses,
                index,
            },
        })
    }

    /// 写成 `.qj`：词按字节序排，字符串进一个 arena，词条 / 释义各一张定长表，外加哈希索引。
    pub fn write_qj(&self, path: &Path, metadata: &Metadata) -> Result<(), GlossaryError> {
        let mut words: Vec<(&str, Translation)> = self.all_entries();
        words.sort_by(|a, b| a.0.cmp(b.0));
        let mut arena = String::new();
        let push = |arena: &mut String, s: &str| -> (u32, u16) {
            let start = arena.len() as u32;
            arena.push_str(s);
            (start, s.len() as u16)
        };
        let mut entries: Vec<EntryRecord> = Vec::with_capacity(words.len());
        let mut senses: Vec<SenseRecord> = Vec::new();
        for (word, translation) in &words {
            let (word_start, word_len) = push(&mut arena, word);
            let sense_start = senses.len() as u32;
            for sense in translation.senses() {
                let (text_start, text_len) = push(&mut arena, &sense.text);
                let (reading_start, reading_len) =
                    push(&mut arena, sense.reading.as_deref().unwrap_or(""));
                let (pos_start, pos_len) = push(
                    &mut arena,
                    sense.part_of_speech.map_or("", PartOfSpeech::abbreviation),
                );
                senses.push(SenseRecord {
                    text_start,
                    reading_start,
                    pos_start,
                    text_len,
                    reading_len,
                    pos_len,
                    reserved: 0,
                });
            }
            entries.push(EntryRecord {
                word_start,
                sense_start,
                word_len,
                sense_count: translation.senses().len() as u16,
            });
        }
        let index = hash::build(entries.len(), |id| {
            let e = &entries[id as usize];
            &arena[e.word_start as usize..e.word_start as usize + usize::from(e.word_len)]
        });
        let metadata = Metadata {
            entries: entries.len() as u64,
            ..metadata.clone()
        };
        let (entries, senses, index) = (
            Table::Owned(entries),
            Table::Owned(senses),
            Table::Owned(index),
        );
        Writer::new(Kind::Glossary, &metadata)?
            .section(TEXT_TAG, arena.as_bytes())
            .section(ENTRIES_TAG, entries.as_bytes())
            .section(SENSES_TAG, senses.as_bytes())
            .section(HASH_TAG, index.as_bytes())
            .write_to(path)?;
        Ok(())
    }

    /// 全部条目（落盘用）。
    fn all_entries(&self) -> Vec<(&str, Translation)> {
        match &self.storage {
            Storage::Owned(map) => map.iter().map(|(k, v)| (k.as_str(), v.clone())).collect(),
            Storage::Mapped { entries, .. } => (0..entries.len() as u32)
                .filter_map(|id| Some((self.mapped_word(id)?, self.mapped_translation(id)?)))
                .collect(),
        }
    }

    pub fn len(&self) -> usize {
        match &self.storage {
            Storage::Owned(map) => map.len(),
            Storage::Mapped { entries, .. } => entries.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 映射表里第 `id` 个词。
    fn mapped_word(&self, id: u32) -> Option<&str> {
        let Storage::Mapped { text, entries, .. } = &self.storage else {
            return None;
        };
        let e = entries.get(id as usize)?;
        text.get(e.word_start as usize..e.word_start as usize + usize::from(e.word_len))
    }

    /// 映射表里第 `id` 个词的译文。
    fn mapped_translation(&self, id: u32) -> Option<Translation> {
        let Storage::Mapped {
            text,
            entries,
            senses,
            ..
        } = &self.storage
        else {
            return None;
        };
        let e = entries.get(id as usize)?;
        let slice = |start: u32, len: u16| -> Option<&str> {
            text.get(start as usize..start as usize + usize::from(len))
        };
        let start = e.sense_start as usize;
        let records = senses.get(start..start + usize::from(e.sense_count))?;
        let senses: Vec<Sense> = records
            .iter()
            .filter_map(|r| {
                let text = slice(r.text_start, r.text_len)?;
                let reading = slice(r.reading_start, r.reading_len)?;
                let pos = slice(r.pos_start, r.pos_len)?;
                Some(Sense {
                    part_of_speech: pos.parse::<PartOfSpeech>().ok(),
                    text: text.to_owned(),
                    reading: (!reading.is_empty()).then(|| reading.to_owned()),
                    fresh: false,
                })
            })
            .collect();
        Some(Translation::new(self.language, senses))
    }
}

/// `v. develop` → 词性 + 译文；识别不出词性缩写时整段都是译文。译文后面 `|` 接的是读音（`n. 開発|かいはつ`）。
pub(crate) fn parse_sense(field: &str) -> Sense {
    let (field, reading) = match field.split_once('|') {
        Some((text, reading)) if !reading.trim().is_empty() => {
            (text.trim(), Some(reading.trim().to_owned()))
        }
        _ => (field, None),
    };
    if let Some((head, rest)) = field.split_once(' ')
        && head.ends_with('.')
        && let Ok(part_of_speech) = head.parse::<PartOfSpeech>()
    {
        return Sense {
            part_of_speech: Some(part_of_speech),
            text: rest.trim().to_owned(),
            reading,
            fresh: false,
        };
    }
    Sense {
        part_of_speech: None,
        text: field.to_owned(),
        reading,
        fresh: false,
    }
}

impl Translator for Glossary {
    fn language(&self) -> Language {
        self.language
    }

    fn translate(&self, text: &str) -> Option<Translation> {
        match &self.storage {
            Storage::Owned(map) => map.get(text).cloned(),
            Storage::Mapped { index, .. } => {
                let id = hash::find(index, text, |id| self.mapped_word(id).unwrap_or(""))?;
                self.mapped_translation(id)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_up_to_two_senses_with_optional_part_of_speech() {
        let glossary = Glossary::parse(
            Language::English,
            "# 注释\n开发\tv. develop\tn. development\tn. exploitation\n中文\tChinese language\n",
        )
        .unwrap();
        let translation = glossary.translate("开发").unwrap();
        assert_eq!(translation.language, Language::English);
        assert_eq!(translation.senses().len(), 2);
        assert_eq!(
            translation.senses()[1].part_of_speech,
            Some(PartOfSpeech::Noun)
        );
        assert_eq!(translation.senses()[1].text, "development");
        let plain = glossary.translate("中文").unwrap();
        assert_eq!(plain.senses()[0].part_of_speech, None);
        assert_eq!(plain.senses()[0].text, "Chinese language");
        assert!(glossary.translate("没有").is_none());
    }

    #[test]
    fn japanese_senses_carry_kana_readings() {
        let glossary = Glossary::parse(
            Language::Japanese,
            "开发\tv. 開発する|かいはつする\tn. 開発\n私\t私|わたし\n",
        )
        .unwrap();
        let senses = glossary.translate("开发").unwrap().senses().to_vec();
        assert_eq!(senses[0].text, "開発する");
        assert_eq!(senses[0].reading.as_deref(), Some("かいはつする"));
        let segments = senses[0].furigana();
        assert_eq!(segments[0].text, "開発");
        assert_eq!(segments[0].reading.as_deref(), Some("かいはつ"));
        assert_eq!(segments[1].text, "する");
        assert_eq!(senses[1].reading, None);
        let plain = glossary.translate("私").unwrap();
        assert_eq!(plain.senses()[0].part_of_speech, None);
        assert_eq!(plain.senses()[0].reading.as_deref(), Some("わたし"));
    }

    #[test]
    fn rejects_line_without_senses() {
        let error = Glossary::parse(Language::English, "开发\n").unwrap_err();
        assert!(matches!(error, GlossaryError::Line { line: 1, .. }));
    }

    #[test]
    fn qj_round_trip_keeps_every_sense() {
        let source = "开发\tv. develop\tv. exploit\n你好\tint. hello\n開発\tn. 開発|かいはつ\n";
        let glossary = Glossary::parse(Language::English, source).unwrap();
        let dir = std::env::temp_dir().join("qingjian-glossary-tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("glossary-{}.qj", std::process::id()));
        glossary
            .write_qj(
                &path,
                &Metadata {
                    name: "测试释义".to_owned(),
                    ..Metadata::default()
                },
            )
            .unwrap();
        let mapped = Glossary::from_path(Language::English, &path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(mapped.len(), 3);
        for word in ["开发", "你好", "開発"] {
            assert_eq!(mapped.translate(word), glossary.translate(word), "{word}");
        }
        assert_eq!(mapped.translate("没有"), None);
        let senses = mapped.translate("開発").unwrap();
        assert_eq!(senses.senses()[0].reading.as_deref(), Some("かいはつ"));
        assert_eq!(senses.senses()[0].part_of_speech, Some(PartOfSpeech::Noun));
    }
}
