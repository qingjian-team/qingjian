//! 生词本：学习语言的译词看过 / 上屏过几次、带它上屏的中文与例句，只写在本机数据目录的 `user-vocab.tsv`。
//!
//! 每次上屏有学习语言说法的中文另记一行时间线（`vocab-log.tsv`，见 [`crate::encounter_log`]）。
//! 输入法记，设置程序的「生词本」页也改（标认识、删除、清空）。两边各开一份、各自原子写整个文件，
//! 所以写之前先 [`VocabularyBook::refresh`]：文件被对方改过就把那边的删除与「认识」合进来，再写，谁也不盖掉谁。

mod entry;
mod file;
mod tracker;

pub use entry::VocabularyEntry;

use std::collections::{BTreeMap, HashSet};
use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};
use std::path::PathBuf;
use std::time::SystemTime;

use jiff::civil::{Date, DateTime};
use qingjian_core::storage::{read_text_lossy, write_atomic_str};
use qingjian_core::{Encounter, FRESH_UNTIL, Language, LevelCount, VocabularySummary};
use qingjian_translate::LevelTable;

use crate::encounter_log::{self, ENCOUNTER_LOG_FILE, LoggedEncounter};
use crate::error::LearningError;
use crate::usage_stats::figures::recent_start;
use file::Key;

/// 文件坏了按行跳过，读不了就只在内存里记（记一次警告），输入不受影响。
#[derive(Debug, Default)]
pub struct VocabularyBook {
    /// (语言代码, 译词) → 记录，按键排好。
    entries: BTreeMap<Key, VocabularyEntry>,

    /// 自上次保存后有没有新记录。
    dirty: bool,

    /// 写回的路径；`None` 只在内存里记。
    path: Option<PathBuf>,

    /// 上次读或写完时文件的样子（修改时间、长度）与里面有哪些词；文件变了说明对方写过。
    synced: Option<(SystemTime, u64)>,

    /// 那时文件里各词的指纹（[`fingerprint`]）：只用来认出对方删了哪些词，不必再存一份词。
    synced_keys: HashSet<u64>,

    /// 各语言的等级表，汇总时按级数词；没有的语言不分级。
    levels: Vec<(Language, LevelTable)>,

    /// 还没追加进时间线文件的相遇。
    encounters: Vec<LoggedEncounter>,

    /// 保存时要从时间线文件里删掉的语言（设置程序清空）。
    cleared: Vec<Language>,
}

impl VocabularyBook {
    /// 从文件加载（不存在就从零开始，保存时建）；读不了就退回只在内存里记。
    pub fn open(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        match read_text_lossy(&path) {
            Ok(text) => {
                let entries = text.as_deref().map(file::parse).unwrap_or_default();
                Self {
                    synced: file::stamp(&path),
                    synced_keys: entries.keys().map(fingerprint).collect(),
                    entries,
                    dirty: false,
                    path: Some(path),
                    levels: Vec::new(),
                    encounters: Vec::new(),
                    cleared: Vec::new(),
                }
            }
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "词汇记录读不了，本次只在内存里记");
                Self::default()
            }
        }
    }

    /// 给某种语言配等级表（`levels-<语言>.tsv`），汇总里就有按级的数字。
    pub fn with_levels(mut self, language: Language, table: LevelTable) -> Self {
        self.levels.retain(|(l, _)| *l != language);
        self.levels.push((language, table));
        self
    }

    /// 记录的译词数（所有语言）。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 某种语言的全部记录，按译词排好。
    pub fn words(&self, language: Language) -> impl Iterator<Item = (&str, &VocabularyEntry)> {
        self.entries
            .iter()
            .filter(move |((code, _), _)| code == language.code())
            .map(|((_, word), entry)| (word.as_str(), entry))
    }

    /// 有记录的语言，按代码排；认不出的代码跳过。
    pub fn languages(&self) -> Vec<Language> {
        let mut languages: Vec<Language> = Vec::new();
        for (code, _) in self.entries.keys() {
            if let Ok(language) = code.parse()
                && !languages.contains(&language)
            {
                languages.push(language);
            }
        }
        languages
    }

    pub fn get(&self, language: Language, word: &str) -> Option<&VocabularyEntry> {
        self.entries.get(&key(language, word))
    }

    fn entry(&mut self, language: Language, word: &str, date: Date) -> &mut VocabularyEntry {
        self.dirty = true;
        let entry = self
            .entries
            .entry(key(language, word))
            .or_insert_with(|| VocabularyEntry::new(date));
        entry.last = date;
        entry
    }

    /// 记一次看到（`record_exposure` 记在今天）。
    pub fn record_exposure_on(&mut self, language: Language, word: &str, date: Date) {
        self.entry(language, word, date).seen += 1;
    }

    /// 记一次上屏（`record_commit` 记在今天）。
    pub fn record_commit_on(&mut self, language: Language, word: &str, used: bool, date: Date) {
        let entry = self.entry(language, word, date);
        entry.committed += 1;
        entry.used += u32::from(used);
    }

    /// 记下带它上屏的中文与那一句，换掉上一次的。
    pub fn record_example_on(
        &mut self,
        language: Language,
        word: &str,
        source: &str,
        example: &str,
        date: Date,
    ) {
        let entry = self.entry(language, word, date);
        entry.source = file::one_line(source);
        entry.example = file::one_line(example);
    }

    /// 标认识或取消；没有这个词返回 `false`。
    pub fn set_known(&mut self, language: Language, word: &str, known: bool) -> bool {
        let Some(entry) = self.entries.get_mut(&key(language, word)) else {
            return false;
        };
        if entry.known != known {
            entry.known = known;
            self.dirty = true;
        }
        true
    }

    /// 从生词本里删掉这个词（之后再看到会从零记起）。
    pub fn remove(&mut self, language: Language, word: &str) -> bool {
        let removed = self.entries.remove(&key(language, word)).is_some();
        self.dirty |= removed;
        removed
    }

    /// 记一次相遇，保存时追加进时间线。
    pub fn record_encounter_at(&mut self, encounter: &Encounter, time: DateTime) {
        self.encounters.push(LoggedEncounter {
            time,
            language: encounter.language,
            text: encounter.text.clone(),
            translation: encounter.translation.clone(),
            reading: encounter.reading.clone().unwrap_or_default(),
        });
    }

    /// 时间线：文件里的加上还没写进去的，按时间先后。
    pub fn encounters(&self, language: Language) -> Vec<LoggedEncounter> {
        let mut all = self
            .log_path()
            .map(|path| encounter_log::read(&path))
            .unwrap_or_default();
        all.extend(self.encounters.iter().cloned());
        all.retain(|encounter| encounter.language == language);
        all
    }

    /// 删掉某种语言的全部记录，连同时间线。
    pub fn clear(&mut self, language: Language) {
        let before = self.entries.len();
        self.entries.retain(|(code, _), _| code != language.code());
        self.dirty |= self.entries.len() != before;
        self.encounters
            .retain(|encounter| encounter.language != language);
        self.cleared.push(language);
    }

    /// 把还没写的相遇追加进时间线；写不进去就留着下次再试。
    pub fn append_encounters(&mut self) -> Result<(), LearningError> {
        let Some(log) = self.log_path() else {
            return Ok(());
        };
        encounter_log::append(&log, &self.encounters)?;
        self.encounters.clear();
        Ok(())
    }

    fn log_path(&self) -> Option<PathBuf> {
        Some(self.path.as_ref()?.with_file_name(ENCOUNTER_LOG_FILE))
    }

    /// 文件被另一方写过时，把那边的改动合进来：那边删掉的词这边也删，「认识」以那边为准，那边新有的词收进来。
    /// 这边自上次同步以来的计数照留。文件没变什么都不做。
    pub fn refresh(&mut self) {
        let Some(path) = &self.path else {
            return;
        };
        let stamp = file::stamp(path);
        if stamp == self.synced {
            return;
        }
        let external = match read_text_lossy(path) {
            Ok(text) => text.as_deref().map(file::parse).unwrap_or_default(),
            Err(error) => {
                tracing::warn!(%error, "词汇记录重读失败，先不合并");
                return;
            }
        };
        let in_file: HashSet<u64> = external.keys().map(fingerprint).collect();
        // 上次同步时文件里有、这次没有的：对方删了
        self.entries.retain(|key, _| {
            let print = fingerprint(key);
            !self.synced_keys.contains(&print) || in_file.contains(&print)
        });
        for (key, theirs) in &external {
            match self.entries.get_mut(key) {
                Some(ours) => ours.known = theirs.known,
                None if !self.synced_keys.contains(&fingerprint(key)) => {
                    self.entries.insert(key.clone(), theirs.clone());
                }
                None => {}
            }
        }
        self.synced = stamp;
        self.synced_keys = in_file;
        // 合并后的内容与文件不同就得写回
        self.dirty = true;
    }

    /// 以 `today` 为准的汇总（`summary` 以本机今天为准）。
    pub fn summary_on(&self, language: Language, today: Date) -> VocabularySummary {
        let recent_from = recent_start(today);
        let table = self
            .levels
            .iter()
            .find(|(l, _)| *l == language)
            .map(|(_, table)| table);
        let mut summary = VocabularySummary::default();
        if let Some(table) = table {
            summary.levels = table
                .levels()
                .iter()
                .enumerate()
                .map(|(rank, name)| LevelCount {
                    name: name.clone(),
                    total: table.size(rank),
                    ..LevelCount::default()
                })
                .collect();
        }
        for (word, entry) in self.words(language) {
            let familiar = entry.seen >= FRESH_UNTIL || entry.known;
            summary.seen += 1;
            summary.familiar += u64::from(familiar);
            summary.committed += u64::from(entry.committed > 0);
            summary.used += u64::from(entry.used > 0);
            summary.new_this_week += u64::from(entry.first >= recent_from && entry.first <= today);
            if let Some(rank) = table.and_then(|table| table.rank(word))
                && let Some(level) = summary.levels.get_mut(rank)
            {
                level.seen += 1;
                level.familiar += u64::from(familiar);
                level.committed += u64::from(entry.committed > 0);
            }
        }
        summary
    }

    /// 写回文件（没有新记录就什么都不做）。写之前不合并，要合并先 [`Self::refresh`]。
    pub fn save(&mut self) -> Result<(), LearningError> {
        let Some(path) = self.path.clone() else {
            return Ok(());
        };
        if let Some(log) = self.log_path() {
            for language in std::mem::take(&mut self.cleared) {
                encounter_log::remove_language(&log, language)?;
            }
        }
        self.append_encounters()?;
        if !self.dirty {
            return Ok(());
        }
        write_atomic_str(&path, &file::render(&self.entries))?;
        self.dirty = false;
        self.synced = file::stamp(&path);
        self.synced_keys = self.entries.keys().map(fingerprint).collect();
        Ok(())
    }
}

/// 词的指纹：进程内固定的哈希，碰撞概率可以忽略。
fn fingerprint(key: &Key) -> u64 {
    BuildHasherDefault::<DefaultHasher>::default().hash_one(key)
}

fn key(language: Language, word: &str) -> Key {
    (language.code().to_owned(), word.to_owned())
}

#[cfg(test)]
mod tests;
