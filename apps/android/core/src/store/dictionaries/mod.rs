//! 词库页：随包的领域词库（`data/dicts/`）与导入的词库（学习数据目录的 `dicts/`），列出、导入、删除。
//! 开关不在这里：开着哪些存在设置里，随键盘选项交给会话（[`crate::Session::set_options`]），会话按文件变化重挂。
//! 导入用 `qingjian_dictionary::import`，与 PC 设置程序同一套（青简 TSV、Rime `.dict.yaml`、只有词的表、`.qj`，都转成 `.qj`；
//! 只有词的表按随包词库派生读音）。

mod entry;
mod page;
mod request;

pub use entry::DictionaryEntry;
pub use page::DictionaryPage;
pub use request::DictionaryRequest;

use std::path::{Path, PathBuf};

use qingjian_dictionary::{Dictionary, DictionaryError, import};

/// 随包领域词库的名字都以这个开头，页面上去掉。
const DOMAIN_PREFIX: &str = "青简领域词库：";

/// 随包领域词库的目录。
pub(crate) fn domains_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("dicts")
}

/// 导入的词库的目录。
pub(crate) fn imported_dir(user_dir: &Path) -> PathBuf {
    user_dir.join("dicts")
}

/// 目录里的 `.qj`，按文件名排，返回 (id, 路径)；目录不存在为空。以 `.` 开头的跳过。
pub(crate) fn files(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<(String, PathBuf)> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "qj"))
        .filter_map(|path| {
            let stem = path.file_stem()?.to_str()?.to_owned();
            (!stem.starts_with('.')).then_some((stem, path))
        })
        .collect();
    files.sort();
    files
}

pub fn dictionaries(
    data_dir: &Path,
    user_dir: &Path,
    request: &DictionaryRequest,
) -> DictionaryPage {
    let mut message = String::new();
    if let Some(source) = &request.import {
        message = import_one(Path::new(source), data_dir, user_dir);
    }
    if let Some(id) = &request.remove {
        message = remove_one(user_dir, id);
    }
    DictionaryPage {
        domains: entries(&domains_dir(data_dir)),
        imported: entries(&imported_dir(user_dir)),
        message,
    }
}

pub fn dictionaries_json(data_dir: &Path, user_dir: &Path, request: &str) -> String {
    let request: DictionaryRequest = serde_json::from_str(request).unwrap_or_else(|error| {
        tracing::warn!(%error, "词库页请求解不开，只列出");
        DictionaryRequest::default()
    });
    serde_json::to_string(&dictionaries(data_dir, user_dir, &request)).unwrap_or_default()
}

fn entries(dir: &Path) -> Vec<DictionaryEntry> {
    files(dir)
        .into_iter()
        .filter_map(|(id, path)| match Dictionary::from_path(&path) {
            Ok(dictionary) => {
                let metadata = dictionary.metadata();
                let name = metadata.map_or(id.as_str(), |m| m.name.as_str());
                Some(DictionaryEntry {
                    name: name.strip_prefix(DOMAIN_PREFIX).unwrap_or(name).to_owned(),
                    license: metadata.map(|m| m.license.clone()).unwrap_or_default(),
                    entries: dictionary.len(),
                    id,
                })
            }
            Err(error) => {
                tracing::warn!(%error, file = %path.display(), "词库读不了，不列");
                None
            }
        })
        .collect()
}

/// 无读音词表的参考词库：随包基础词库加随包领域词库，读不到的跳过。
fn reference_dictionaries(data_dir: &Path) -> Vec<Dictionary> {
    let mut references = Vec::new();
    let mut read = |path: &Path| match Dictionary::from_path(path) {
        Ok(dictionary) => references.push(dictionary),
        Err(error) => tracing::warn!(path = %path.display(), %error, "参考词库打不开"),
    };
    let packed = data_dir.join("dict.qj");
    let base = if packed.is_file() {
        packed
    } else {
        data_dir.join("dict.tsv")
    };
    read(&base);
    for (_, path) in files(&domains_dir(data_dir)) {
        read(&path);
    }
    references
}

fn import_one(source: &Path, data_dir: &Path, user_dir: &Path) -> String {
    let references = reference_dictionaries(data_dir);
    let borrowed: Vec<&Dictionary> = references.iter().collect();
    match import::import_dictionary(source, &imported_dir(user_dir), &borrowed) {
        Ok(done) => {
            let imported = &done.imported;
            tracing::info!(name = %imported.name, entries = imported.entries, "词库已导入");
            let mut message = format!("已导入「{}」，共 {} 条", imported.name, imported.entries);
            if let Some(report) = &done.report
                && !report.unknown.is_empty()
            {
                message.push_str(&format!(
                    "；{} 条跳过（字不在随包词库）",
                    report.unknown.len()
                ));
            }
            message
        }
        Err(error) => {
            tracing::warn!(%error, file = %source.display(), "词库导入失败");
            match error {
                DictionaryError::CodeWords => {
                    "导入失败：这是五笔词库（第二列是编码），手机上还不能导入五笔词库".to_owned()
                }
                DictionaryError::NoWordEntries => {
                    "导入失败：词表里的字都不在随包词库，推不出读音".to_owned()
                }
                DictionaryError::Corrupt(_) | DictionaryError::Line { .. } => {
                    "导入失败：没有可用的词条，每行应是「词<Tab>拼音<Tab>词频」，也可以只有词（读音按随包词库补）"
                        .to_owned()
                }
                _ => "导入失败：文件读不了，或不是词库文件".to_owned(),
            }
        }
    }
}

fn remove_one(user_dir: &Path, id: &str) -> String {
    let Some((_, path)) = files(&imported_dir(user_dir))
        .into_iter()
        .find(|(stem, _)| stem == id)
    else {
        return String::new();
    };
    match std::fs::remove_file(&path) {
        Ok(()) => "已删除".to_owned(),
        Err(error) => {
            tracing::warn!(%error, file = %path.display(), "词库删不掉");
            "删除失败".to_owned()
        }
    }
}

#[cfg(test)]
mod tests;
