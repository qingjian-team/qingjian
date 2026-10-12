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

/// 剥德语名词译词开头的定冠词（冠词闸门未过时的兜底形态：名词保留大写与词性）。
/// 只剥「冠词 + 空格 + 词」；单独的 der/die/das 不当冠词剥。
fn strip_noun_article(sense: &str) -> String {
    for article in ["der ", "die ", "das "] {
        if let Some(rest) = sense.strip_prefix(article) {
            return rest.to_owned();
        }
    }
    sense.to_owned()
}

/// 导出成输入法加载的学习语言表：`词\t词性 译词\t词性 译词`，日文译词后面接 `|假名`。同一个词以最后一条为准，按词排序。
/// `strip_german_articles` 时德语表剥掉名词冠词（名词保留大写与词性），其他语言不变。
pub fn export(input: &Path, out_dir: &Path, strip_german_articles: bool) -> Result<(), GlossError> {
    let mut latest: BTreeMap<String, GlossEntry> = BTreeMap::new();
    for entry in read_entries::<GlossEntry>(input)? {
        latest.insert(entry.word.clone(), entry);
    }
    std::fs::create_dir_all(out_dir)?;
    let mut english = Vec::new();
    let mut japanese = Vec::new();
    let mut vietnamese = Vec::new();
    let mut german = Vec::new();
    let (mut en_count, mut ja_count, mut vi_count, mut de_count) = (0, 0, 0, 0);
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
        if !entry.vi.is_empty() {
            write!(vietnamese, "{}", entry.word)?;
            for sense in &entry.vi {
                write!(vietnamese, "\t{pos}{sense}")?;
            }
            writeln!(vietnamese)?;
            vi_count += 1;
        }
        if !entry.de.is_empty() {
            let is_noun = entry.pos.as_deref() == Some("n.");
            write!(german, "{}", entry.word)?;
            for sense in &entry.de {
                let text = if strip_german_articles && is_noun {
                    strip_noun_article(sense)
                } else {
                    sense.clone()
                };
                write!(german, "\t{pos}{text}")?;
            }
            writeln!(german)?;
            de_count += 1;
        }
    }
    write_if_non_empty(
        out_dir,
        "glossary-en.tsv",
        "# 由 qingjian-gloss-gen 生成（LLM）。词\t[词性. ]译词\t[词性. ]译词",
        &english,
    )?;
    write_if_non_empty(
        out_dir,
        "glossary-ja.tsv",
        "# 由 qingjian-gloss-gen 生成（LLM）。词\t[词性. ]译词|假名\t[词性. ]译词|假名",
        &japanese,
    )?;
    write_if_non_empty(
        out_dir,
        "glossary-vi.tsv",
        "# 由 qingjian-gloss-gen 生成（LLM）。词\t[词性. ]译词\t[词性. ]译词",
        &vietnamese,
    )?;
    write_if_non_empty(
        out_dir,
        "glossary-de.tsv",
        "# 由 qingjian-gloss-gen 生成（LLM）。词\t[词性. ]译词\t[词性. ]译词",
        &german,
    )?;
    tracing::info!(
        words = latest.len(),
        english = en_count,
        japanese = ja_count,
        vietnamese = vi_count,
        german = de_count,
        out_dir = %out_dir.display(),
        "导出完成"
    );
    Ok(())
}

fn write_if_non_empty(
    out_dir: &Path,
    name: &str,
    header: &str,
    body: &[u8],
) -> Result<(), GlossError> {
    if body.is_empty() {
        return Ok(());
    }
    let path = out_dir.join(name);
    let mut out = BufWriter::new(File::create(path)?);
    writeln!(out, "{header}")?;
    out.write_all(body)?;
    out.flush()?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::JapaneseSense;

    fn german_entry(word: &str, pos: Option<&str>, de: &[&str]) -> GlossEntry {
        GlossEntry {
            word: word.to_owned(),
            pos: pos.map(str::to_owned),
            en: Vec::new(),
            ja: Vec::<JapaneseSense>::new(),
            vi: Vec::new(),
            de: de.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    #[test]
    fn export_german_with_and_without_articles() {
        let dir = std::env::temp_dir().join(format!("qj-export-de-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("gloss-llm.jsonl");
        std::fs::write(
            &input,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&german_entry("学校", Some("n."), &["die Schule"])).unwrap(),
                serde_json::to_string(&german_entry("在", Some("prep."), &["in"])).unwrap(),
            ),
        )
        .unwrap();

        // 默认保留冠词
        export(&input, &dir, false).unwrap();
        let with = std::fs::read_to_string(dir.join("glossary-de.tsv")).unwrap();
        assert!(with.lines().any(|l| l == "学校\tn. die Schule"));
        assert!(with.lines().any(|l| l == "在\tprep. in"));

        // 兜底形态：名词剥冠词留大写，非名词不动
        let stripped_dir = dir.join("stripped");
        export(&input, &stripped_dir, true).unwrap();
        let stripped = std::fs::read_to_string(stripped_dir.join("glossary-de.tsv")).unwrap();
        assert!(stripped.lines().any(|l| l == "学校\tn. Schule"));
        assert!(!stripped.lines().any(|l| l.contains("die Schule")));
        assert!(stripped.lines().any(|l| l == "在\tprep. in"));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn export_german_reads_pilot_jsonl_with_only_de() {
        // pilot 期的 JSONL 只有 word/pos/de（en/ja/vi 缺省），serde default 要能读
        let dir = std::env::temp_dir().join(format!("qj-export-de-pilot-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("gloss-llm.jsonl");
        std::fs::write(
            &input,
            "{\"word\":\"房子\",\"pos\":\"n.\",\"de\":[\"das Haus\"]}\n",
        )
        .unwrap();
        export(&input, &dir, false).unwrap();
        let body = std::fs::read_to_string(dir.join("glossary-de.tsv")).unwrap();
        assert!(body.lines().any(|l| l == "房子\tn. das Haus"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
