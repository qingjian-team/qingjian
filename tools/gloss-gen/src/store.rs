//! JSONL 结果文件：逐行追加、启动时读回已完成的词、导出成释义表。

use std::collections::{BTreeMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::marker::PhantomData;
use std::path::Path;
use std::sync::Mutex;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::entry::{EnglishGlossEntry, GlossEntry, PinyinEntry};
use crate::error::GlossError;

/// JSONL 里的一行：按词去重续跑。
pub trait Keyed: Serialize + DeserializeOwned {
    /// 这条结果对应的词。
    fn key(&self) -> &str;
}

impl Keyed for GlossEntry {
    fn key(&self) -> &str {
        &self.word
    }
}

impl Keyed for PinyinEntry {
    fn key(&self) -> &str {
        &self.word
    }
}

impl Keyed for EnglishGlossEntry {
    fn key(&self) -> &str {
        &self.word
    }
}

/// 打开着的结果文件。
pub struct Store<T> {
    /// 已有的词。
    done: HashSet<String>,

    /// 追加写入端。
    file: Mutex<File>,

    /// 条目类型。
    _entry: PhantomData<T>,
}

impl<T: Keyed> Store<T> {
    /// 打开（没有就建）并读回已有的条目。
    pub fn open(path: &Path) -> Result<Self, GlossError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let done: HashSet<String> = if path.exists() {
            read_entries::<T>(path)?
                .into_iter()
                .map(|e| e.key().to_owned())
                .collect()
        } else {
            HashSet::new()
        };
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            done,
            file: Mutex::new(file),
            _entry: PhantomData,
        })
    }

    pub fn contains(&self, word: &str) -> bool {
        self.done.contains(word)
    }

    pub fn len(&self) -> usize {
        self.done.len()
    }

    /// 追加一批条目并落盘。
    pub fn append(&self, entries: &[T]) -> Result<(), GlossError> {
        let mut file = self.file.lock().unwrap_or_else(|e| e.into_inner());
        for entry in entries {
            serde_json::to_writer(&mut *file, entry)?;
            file.write_all(b"\n")?;
        }
        file.flush()?;
        Ok(())
    }
}

/// 读全部条目，坏行跳过并记一条警告。
pub fn read_entries<T: Keyed>(path: &Path) -> Result<Vec<T>, GlossError> {
    let reader = BufReader::new(File::open(path)?);
    let mut entries = Vec::new();
    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<T>(&line) {
            Ok(entry) => entries.push(entry),
            Err(error) => tracing::warn!(line = index + 1, %error, "跳过坏行"),
        }
    }
    Ok(entries)
}

/// 导出成输入法加载的两张表：`词\t词性 译词\t词性 译词`，日文译词后面接 `|假名`。同一个词以最后一条为准，按词排序。
pub fn export(input: &Path, out_dir: &Path) -> Result<(), GlossError> {
    let mut latest: BTreeMap<String, GlossEntry> = BTreeMap::new();
    for entry in read_entries::<GlossEntry>(input)? {
        latest.insert(entry.word.clone(), entry);
    }
    std::fs::create_dir_all(out_dir)?;
    let mut english = BufWriter::new(File::create(out_dir.join("glossary-en.tsv"))?);
    let mut japanese = BufWriter::new(File::create(out_dir.join("glossary-ja.tsv"))?);
    writeln!(
        english,
        "# 由 qingjian-gloss-gen 生成（LLM）。词\t[词性. ]译词\t[词性. ]译词"
    )?;
    writeln!(
        japanese,
        "# 由 qingjian-gloss-gen 生成（LLM）。词\t[词性. ]译词|假名\t[词性. ]译词|假名"
    )?;
    let (mut en_count, mut ja_count) = (0, 0);
    for entry in latest.values() {
        let pos = entry
            .pos
            .as_deref()
            .map(|p| format!("{p} "))
            .unwrap_or_default();
        if !entry.en.is_empty() {
            write!(english, "{}", entry.word)?;
            for sense in &entry.en {
                write!(english, "\t{pos}{sense}")?;
            }
            writeln!(english)?;
            en_count += 1;
        }
        if !entry.ja.is_empty() {
            write!(japanese, "{}", entry.word)?;
            for sense in &entry.ja {
                match &sense.reading {
                    Some(reading) => write!(japanese, "\t{pos}{}|{reading}", sense.text)?,
                    None => write!(japanese, "\t{pos}{}", sense.text)?,
                }
            }
            writeln!(japanese)?;
            ja_count += 1;
        }
    }
    english.flush()?;
    japanese.flush()?;
    tracing::info!(
        words = latest.len(),
        english = en_count,
        japanese = ja_count,
        out_dir = %out_dir.display(),
        "导出完成"
    );
    Ok(())
}

/// 英→中释义导出成 `glossary-zh.tsv`：`词\t词性. 释义\t…`，键是小写（英文候选按敲的大小写显示，查表时统一小写）。
/// 同一个词以最后一条为准，按词排序。
pub fn export_english(input: &Path, out_dir: &Path) -> Result<(), GlossError> {
    let mut latest: BTreeMap<String, EnglishGlossEntry> = BTreeMap::new();
    for entry in read_entries::<EnglishGlossEntry>(input)? {
        latest.insert(entry.word.to_ascii_lowercase(), entry);
    }
    std::fs::create_dir_all(out_dir)?;
    let path = out_dir.join("glossary-zh.tsv");
    let mut out = BufWriter::new(File::create(&path)?);
    writeln!(
        out,
        "# 由 qingjian-gloss-gen english 生成（LLM）：英文词的中文释义。词（小写）\t[词性. ]释义\t[词性. ]释义"
    )?;
    let mut count = 0;
    for (key, entry) in &latest {
        if entry.zh.is_empty() {
            continue;
        }
        let pos = entry
            .pos
            .as_deref()
            .map(|p| format!("{p} "))
            .unwrap_or_default();
        write!(out, "{key}")?;
        for sense in &entry.zh {
            write!(out, "\t{pos}{sense}")?;
        }
        writeln!(out)?;
        count += 1;
    }
    out.flush()?;
    tracing::info!(words = latest.len(), exported = count, out = %path.display(), "英→中释义导出完成");
    Ok(())
}
