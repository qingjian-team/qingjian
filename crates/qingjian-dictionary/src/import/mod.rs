//! 导入词库：把用户给的文件（青简 TSV、Rime `.dict.yaml`、现成的 `.qj`）变成用户词库目录里的一个 `.qj`；
//! 另有 [`import_word_list`] 收「只有词、没有拼音」的词表，读音从参考词库取（见 `word_list` 模块）。
//!
//! 导入时只做格式转换，不做拼音校验：不合法的音节查不到而已，不影响别的词。
//! 第二列是形码编码的（五笔词库）不进拼音词库，报 [`DictionaryError::CodeWords`]，由调用方改走 [`import_code_words`]。
//! 目标文件名取源文件的主干（`law.dict.yaml` → `law.qj`），已存在就覆盖（重新导入即更新）。

mod code_words;
mod format;
mod imported;
mod rime;
mod word_list;

use std::path::{Path, PathBuf};

use qingjian_format::{Container, Metadata};

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
    let (dictionary, metadata) = if Container::is_qj(source) {
        let dictionary = Dictionary::open_qj(source)?;
        let metadata = dictionary.metadata().cloned().unwrap_or_default();
        (dictionary, metadata)
    } else {
        let text = std::fs::read_to_string(source)?;
        let (tsv, name) = if rime::looks_like_rime(&text) {
            let parsed = rime::to_tsv(&text);
            (parsed.tsv, parsed.name)
        } else {
            (text, None)
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
    for suffix in [".dict.yaml", ".yaml", ".yml", ".tsv", ".txt", ".qj"] {
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
mod tests {
    use super::*;

    #[test]
    fn imports_tsv_and_rime_into_qj() {
        let dir = std::env::temp_dir().join(format!("qingjian-import-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let tsv = dir.join("finance.tsv");
        std::fs::write(&tsv, "账套\tzhang tao\t500\n").unwrap();
        let imported = import(&tsv, &dir.join("dicts")).unwrap();
        assert_eq!(imported.name, "finance");
        assert_eq!(imported.entries, 1);
        let dictionary = Dictionary::from_path(&imported.path).unwrap();
        assert_eq!(dictionary.lookup(&["zhang", "tao"], false)[0].text, "账套");
        assert_eq!(dictionary.metadata().unwrap().name, "finance");

        let rime = dir.join("law.dict.yaml");
        std::fs::write(
            &rime,
            "# Rime dictionary\n---\nname: law\nversion: \"1\"\nsort: by_weight\n...\n合同法\the tong fa\t120\n民法典\tmin fa dian\n",
        )
        .unwrap();
        let imported = import(&rime, &dir.join("dicts")).unwrap();
        assert_eq!(imported.path.file_name().unwrap(), "law.qj");
        assert_eq!(imported.name, "law");
        assert_eq!(imported.entries, 2);
        let dictionary = Dictionary::from_path(&imported.path).unwrap();
        assert_eq!(
            dictionary.lookup(&["min", "fa", "dian"], false)[0].text,
            "民法典"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn imports_tone_numbered_pinyin() {
        let dir =
            std::env::temp_dir().join(format!("qingjian-import-tones-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let tsv = dir.join("toned.tsv");
        std::fs::write(&tsv, "东西\tdong1 xi5\t20\n绿色\tlv4 se4\t10\n").unwrap();
        assert!(!format::is_code_table(
            &std::fs::read_to_string(&tsv).unwrap()
        ));
        let imported = import(&tsv, &dir.join("dicts")).unwrap();
        let dictionary = Dictionary::from_path(&imported.path).unwrap();
        let hit = dictionary.lookup(&["dong", "xi"], false)[0];
        assert_eq!((hit.text, hit.pinyin), ("东西", "dong xi"));
        assert_eq!(hit.tones.to_vec(2), [1, 5]);
        assert_eq!(dictionary.lookup(&["lv", "se"], false)[0].text, "绿色");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn refuses_wubi_words_as_pinyin() {
        let dir = std::env::temp_dir().join(format!("qingjian-import-wubi-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let tsv = dir.join("wubi-test.tsv");
        std::fs::write(&tsv, "拉混\truji\t1\n青简\tgetu\t1\n").unwrap();
        assert!(matches!(
            import(&tsv, &dir.join("dicts")),
            Err(DictionaryError::CodeWords)
        ));
        assert!(!dir.join("dicts/wubi-test.qj").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn imports_word_list_deriving_readings_from_the_reference() {
        let dir =
            std::env::temp_dir().join(format!("qingjian-import-words-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let reference = Dictionary::parse("李\tli\t900\n三\tsan\t900\n").unwrap();
        let list = dir.join("names.txt");
        std::fs::write(&list, "李三\n囧三\n").unwrap();

        let imported = import_word_list(&list, &dir.join("dicts"), &[&reference]).unwrap();
        assert_eq!(imported.imported.path.file_name().unwrap(), "names.qj");
        assert_eq!(imported.imported.name, "names");
        assert_eq!(imported.imported.entries, 1);
        assert_eq!(imported.report.words, 1);
        assert_eq!(imported.report.unknown, ["囧三".to_owned()]);
        let dictionary = Dictionary::from_path(&imported.imported.path).unwrap();
        assert_eq!(dictionary.lookup(&["li", "san"], false)[0].text, "李三");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_word_list_without_any_derivable_word() {
        let dir =
            std::env::temp_dir().join(format!("qingjian-import-names-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let reference = Dictionary::parse("三\tsan\t900\n").unwrap();
        let list = dir.join("names.txt");
        std::fs::write(&list, "囧囧\n").unwrap();

        assert!(matches!(
            import_word_list(&list, &dir.join("dicts"), &[&reference]),
            Err(DictionaryError::NoWordEntries)
        ));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn strips_known_extensions() {
        assert_eq!(strip_extensions("law.dict.yaml"), "law");
        assert_eq!(strip_extensions("dict.tsv"), "dict");
        assert_eq!(strip_extensions("x.qj"), "x");
        assert_eq!(strip_extensions("noext"), "noext");
    }

    /// 建一个临时目录，跑完删掉。
    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("qingjian-import-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn auto_import_derives_readings_for_word_lists() {
        let dir = scratch("auto-words");
        let reference = Dictionary::parse("李\tli\t900\n三\tsan\t900\n").unwrap();
        // 只有词的 txt：全部按参考词库派生
        let list = dir.join("names.txt");
        std::fs::write(&list, "李三\n囧三\n").unwrap();
        let done = import_dictionary(&list, &dir.join("dicts"), &[&reference]).unwrap();
        assert_eq!(done.imported.entries, 1);
        assert_eq!(done.report.unwrap().unknown, ["囧三".to_owned()]);

        // Rime 词库混了有读音的行：没读音的派生，写明的原样保留
        let rime = dir.join("mixed.dict.yaml");
        std::fs::write(&rime, "---\nname: mixed\n...\n李三\n三\tsan\t5\n").unwrap();
        let done = import_dictionary(&rime, &dir.join("dicts"), &[&reference]).unwrap();
        assert_eq!(done.imported.entries, 2);
        let dictionary = Dictionary::from_path(&done.imported.path).unwrap();
        assert_eq!(dictionary.lookup(&["li", "san"], false)[0].text, "李三");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn auto_import_keeps_normal_dictionaries_on_the_plain_path() {
        let dir = scratch("auto-plain");
        let reference = Dictionary::parse("李\tli\t900\n三\tsan\t900\n").unwrap();
        // 每行都有读音：仍走普通导入，返回的 report 为空
        let rime = dir.join("law.dict.yaml");
        std::fs::write(&rime, "---\nname: law\n...\n合同法\the tong fa\t120\n").unwrap();
        let done = import_dictionary(&rime, &dir.join("dicts"), &[&reference]).unwrap();
        assert!(done.report.is_none());
        assert_eq!(done.imported.name, "law");

        // 混进来的带数字声调的行也照原样保留（词表按无读音处理时同样认）
        let list = dir.join("toned.txt");
        std::fs::write(&list, "东西\tdong1 xi5\t20\n李三\n").unwrap();
        let done = import_dictionary(&list, &dir.join("dicts"), &[&reference]).unwrap();
        let dictionary = Dictionary::from_path(&done.imported.path).unwrap();
        assert_eq!(
            dictionary.lookup(&["dong", "xi"], false)[0].pinyin,
            "dong xi"
        );
        assert_eq!(dictionary.lookup(&["li", "san"], false)[0].text, "李三");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
