//! 「高级」页的学习数据：导出哪些文件、清空删哪些文件。不要句柄：清空前壳先关掉开着的引擎（学习数据落盘），删完再装回来，
//! 否则内存里的旧数据会在下次落盘时写回去。

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use qingjian_learning::{ENCOUNTER_LOG_FILE, FrequencyLearner};

/// 词频文件（[`crate::assemble`] 从这里读学习数据），其余各表是它同目录的侧车。
const FREQUENCY_FILE: &str = "user.tsv";

/// 生词本（`VocabularyBook` 的文件）。
const VOCABULARY_FILE: &str = "user-vocab.tsv";

/// 导出的文件：学习数据目录顶层的 `.tsv` 与 `.json`（输入习惯、生词本与时间线、输入统计、常用语、面板常用、个人释义表）。
/// 剪贴板历史（`clipboard.jsonl` 与图片）可能有密码、验证码一类内容，不导出，同 PC 导出日志时抹掉密钥；导入的词库是用户手上本来就有的文件，也不带。
/// 返回 (压缩包里的名字, 路径)，按名字排。
pub fn export_files(user_dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(user_dir) else {
        return Vec::new();
    };
    let mut files: Vec<(String, PathBuf)> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| path.extension().is_some_and(|e| e == "tsv" || e == "json"))
        .filter_map(|path| Some((path.file_name()?.to_str()?.to_owned(), path)))
        .collect();
    files.sort();
    files
}

/// 删掉学习数据：`habits` 是输入习惯（词频、造的词、个人 n-gram、按输入串的选择、个人英文词、敲错表），
/// `vocabulary` 是生词本与它的时间线（全部语言，同 PC 生词本页的「清空」）。输入统计、常用语、剪贴板、词库与设置不动。
/// 返回删掉的文件数；没有的文件不算错，删不掉的记日志跳过。
pub fn clear(user_dir: &Path, habits: bool, vocabulary: bool) -> usize {
    let mut targets = Vec::new();
    if habits {
        targets.extend(FrequencyLearner::files(&user_dir.join(FREQUENCY_FILE)));
    }
    if vocabulary {
        targets.push(user_dir.join(VOCABULARY_FILE));
        targets.push(user_dir.join(ENCOUNTER_LOG_FILE));
    }
    let mut removed = 0;
    for path in targets {
        match std::fs::remove_file(&path) {
            Ok(()) => removed += 1,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => tracing::warn!(path = %path.display(), %error, "学习数据删不掉"),
        }
    }
    tracing::info!(habits, vocabulary, removed, "清空学习数据");
    removed
}

/// [`export_files`] 的 JSON：`[{"name":…,"path":…}]`。
pub(crate) fn export_files_json(user_dir: &Path) -> String {
    let files: Vec<serde_json::Value> = export_files(user_dir)
        .into_iter()
        .map(|(name, path)| serde_json::json!({ "name": name, "path": path.to_string_lossy() }))
        .collect();
    serde_json::Value::Array(files).to_string()
}
