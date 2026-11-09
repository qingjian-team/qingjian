//! 导入词库：把用户给的文件（青简 TSV、Rime `.dict.yaml`、现成的 `.qj` 与压缩的 `.qjz`）变成用户词库目录里的一个 `.qj`；
//! 另有 [`import_word_list`] 收「只有词、没有拼音」的词表，读音从参考词库取（见 `word_list` 模块）。
//!
//! 导入时只做格式转换，不做拼音校验：不合法的音节查不到而已，不影响别的词。
//! 第二列是形码编码的（五笔词库）不进拼音词库，报 [`DictionaryError::CodeWords`]，由调用方改走 [`import_code_words`]。
//! 目标文件名取源文件的主干（`law.dict.yaml` → `law.qj`），已存在就覆盖（重新导入即更新）。

mod code_words;
mod format;
mod imported;
mod rime;
pub mod scel;
mod word_list;

use std::path::{Path, PathBuf};

use qingjian_format::{Container, Kind, Metadata, qjz};

use crate::dictionary::Dictionary;
use crate::error::DictionaryError;

pub use code_words::{code_words_scheme, import_code_words};
pub use format::is_code_table;
pub use imported::Imported;
// `dict-convert wubi` 也读 Rime `.dict.yaml`（形码码表的第二列是编码，转换方式一样），共用这一个解析器
pub use rime::{Parsed, looks_like_rime, to_tsv};
// CLI 的 `--import-words` 预览直接用这两个：解析词表、派生读音
pub use word_list::{Derived, Report, Word, derive, parse_words};

/// 导入无读音词表的结果：写出的文件 + 派生统计。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedWordList {
    /// 写出的 `.qj`。
    pub imported: Imported,

    /// 派生统计（跳过的词都在里面）。
    pub report: Report,
}

/// 导入一本词库的结果：写出的文件；无读音词表另带派生统计（普通词库为 `None`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedDictionary {
    /// 写出的 `.qj`。
    pub imported: Imported,

    /// 无读音词表的派生统计；普通词库为 `None`。
    pub report: Option<Report>,
}

/// 导入一本词库：源文件里有没写读音的词（只有词的 `.txt` / 混了无读音行的 Rime 词库）时，
/// 按 [`import_word_list`] 从 `references`（随包基础词库与领域词库）派生读音；全部带读音或 `.qj` 时同 [`import`]。
/// 判据只看文本：读不出文本的（编码不是 UTF-8 等）也交给 [`import`] 去报它自己的错。
pub fn import_dictionary(
    source: &Path,
    dest_dir: &Path,
    references: &[&Dictionary],
) -> Result<ImportedDictionary, DictionaryError> {
    let needs_readings = !Container::is_qj(source)
        && !qjz::is_qjz(source)
        && std::fs::read_to_string(source).is_ok_and(|text| has_words_without_reading(&text));
    if needs_readings {
        let done = import_word_list(source, dest_dir, references)?;
        Ok(ImportedDictionary {
            imported: done.imported,
            report: Some(done.report),
        })
    } else {
        Ok(ImportedDictionary {
            imported: import(source, dest_dir)?,
            report: None,
        })
    }
}

/// 词表里有没写读音的词（`词`、`词<Tab>词频`、Rime 里没拼音的行）：这种源文件不能按普通词库解析。
fn has_words_without_reading(text: &str) -> bool {
    parse_words(text).iter().any(|word| word.reading.is_none())
}

/// 把 `source` 导入到 `dest_dir`，返回写出的文件与元数据。
pub fn import(source: &Path, dest_dir: &Path) -> Result<Imported, DictionaryError> {
    let stem = stem_of(source)?;
    std::fs::create_dir_all(dest_dir)?;
    let target = dest_dir.join(format!("{stem}.qj"));
    if qjz::is_qjz(source) {
        return import_qjz(source, &stem, target);
    }
    let (dictionary, metadata) = if Container::is_qj(source) {
        let dictionary = Dictionary::open_qj(source)?;
        let metadata = dictionary.metadata().cloned().unwrap_or_default();
        (dictionary, metadata)
    } else {
        let bytes = std::fs::read(source)?;
        // 搜狗细胞词库是二进制：先按魔数 / 扩展名认出来，转成 TSV 再走同一条解析
        if scel::is_scel(&source.to_string_lossy(), &bytes) {
            let tsv =
                scel::to_tsv(&bytes).ok_or(DictionaryError::Corrupt(".scel 里没有可解析的词条"))?;
            let dictionary = Dictionary::parse(&tsv)?;
            let metadata = Metadata {
                name: stem.clone(),
                source: source.display().to_string(),
                ..Metadata::default()
            };
            return finish_import(dictionary, metadata, target);
        }
        let text = String::from_utf8(bytes)
            .map_err(|_| DictionaryError::Corrupt("file is neither text nor .scel"))?;
        // 带 BOM 的 Rime 词库认不出 YAML 头，整份按 TSV 读、报第 1 行缺拼音
        let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
        let (tsv, name) = if rime::looks_like_rime(text) {
            let parsed = rime::to_tsv(text);
            (parsed.tsv, parsed.name)
        } else {
            (text.to_owned(), None)
        };
        if format::is_code_table(&tsv) {
            return Err(DictionaryError::CodeWords);
        }
        let dictionary = Dictionary::parse(&tsv)?;
        let metadata = Metadata {
            name: name.unwrap_or_else(|| stem.clone()),
            source: source.display().to_string(),
            ..Metadata::default()
        };
        (dictionary, metadata)
    };
    finish_import(dictionary, metadata, target)
}

/// `.qjz` 先解到系统临时目录，再按 `.qj` 导入；解出来不是拼音词库（释义表等）就拒绝。
/// 不直接解进词库目录：输入法会扫那里的 `.qj`，半成品不能让它看见。
fn import_qjz(source: &Path, stem: &str, target: PathBuf) -> Result<Imported, DictionaryError> {
    let unpacked =
        std::env::temp_dir().join(format!("qingjian-import-{}-{stem}.qj", std::process::id()));
    let result = qjz::unpack(source, &unpacked, Kind::Dictionary)
        .map_err(DictionaryError::from)
        .and_then(|_| {
            let dictionary = Dictionary::open_qj(&unpacked)?;
            let metadata = dictionary.metadata().cloned().unwrap_or_default();
            finish_import(dictionary, metadata, target)
        });
    let _ = std::fs::remove_file(&unpacked);
    result
}

/// 导入无读音词表：每行 `词` / `词<Tab>词频` / `词<Tab>读音[<Tab>词频]`（口径见 `word_list` 模块），
/// 读音按 `references`（随包基础词库与领域词库）取：词表自带读音优先，其次整词词条，最后最长匹配切分。
/// 写出的 `.qj` 与 [`import`] 同一套命名。
pub fn import_word_list(
    source: &Path,
    dest_dir: &Path,
    references: &[&Dictionary],
) -> Result<ImportedWordList, DictionaryError> {
    let stem = stem_of(source)?;
    std::fs::create_dir_all(dest_dir)?;
    let target = dest_dir.join(format!("{stem}.qj"));
    let words = parse_words(&std::fs::read_to_string(source)?);
    let derived = derive(&words, references);
    if derived.report.words == 0 {
        return Err(DictionaryError::NoWordEntries);
    }
    let dictionary = Dictionary::parse(&derived.tsv)?;
    let metadata = Metadata {
        name: stem,
        source: source.display().to_string(),
        ..Metadata::default()
    };
    Ok(ImportedWordList {
        imported: finish_import(dictionary, metadata, target)?,
        report: derived.report,
    })
}

/// 源文件名的主干；没有可用的文件名就报错。
fn stem_of(source: &Path) -> Result<String, DictionaryError> {
    source
        .file_name()
        .and_then(|n| n.to_str())
        .map(strip_extensions)
        .filter(|s| !s.is_empty())
        .ok_or(DictionaryError::Corrupt("source has no usable file name"))
}

/// 空词库拒绝（那种文件不是词库），非空就写 `.qj`。
fn finish_import(
    dictionary: Dictionary,
    metadata: Metadata,
    target: PathBuf,
) -> Result<Imported, DictionaryError> {
    if dictionary.is_empty() {
        return Err(DictionaryError::Corrupt(
            "no usable entries; expected word and explicit pinyin columns",
        ));
    }
    dictionary.write_qj(&target, &metadata)?;
    Ok(Imported {
        path: target,
        name: metadata.name,
        entries: dictionary.len(),
    })
}

/// `law.dict.yaml` → `law`，`dict.tsv` → `dict`。
pub(crate) fn strip_extensions(file_name: &str) -> String {
    let mut stem = file_name;
    for suffix in [
        ".dict.yaml",
        ".yaml",
        ".yml",
        ".tsv",
        ".txt",
        ".scel",
        ".qjz",
        ".qj",
    ] {
        if let Some(s) = stem.strip_suffix(suffix) {
            stem = s;
            break;
        }
    }
    stem.to_owned()
}

/// 词库目录里的 `.qj` 文件名。
pub fn target_name(stem: &str) -> PathBuf {
    PathBuf::from(format!("{stem}.qj"))
}

#[cfg(test)]
mod tests;
