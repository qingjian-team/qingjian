//! 附加词库：随包的领域词库（`Resources/dicts/`，按 `[dictionaries] domains` 挑）与用户目录 `dicts/` 下的 `.qj`（或 TSV）
//! 文件（按 `[dictionaries] disabled` 过滤），加载后一起接到 Engine 上。

use std::path::{Path, PathBuf};

use qingjian_dictionary::Dictionary;
use qingjian_platform::DictionariesConfig;

/// 目录里能加载的扩展名。
const EXTENSIONS: [&str; 2] = ["qj", "tsv"];

/// 列出目录里的词库文件（按文件名排序），返回 (文件名不含扩展名, 路径)。目录不存在就是空。
pub fn list(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<(String, PathBuf)> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| EXTENSIONS.contains(&e))
        })
        .filter_map(|p| {
            let stem = p.file_stem()?.to_str()?.to_owned();
            Some((stem, p))
        })
        .collect();
    files.sort();
    files
}

/// 加载没被关掉的词库：先随包领域词库，再用户目录。坏文件只记日志、跳过：一本词库坏了不能拖垮输入法。
pub fn load(
    bundled_dir: Option<&Path>,
    user_dir: Option<&Path>,
    config: &DictionariesConfig,
) -> Vec<Dictionary> {
    let mut loaded = Vec::new();
    if let Some(dir) = bundled_dir {
        for (stem, path) in list(dir) {
            if !config.is_domain_enabled(&stem) {
                tracing::debug!(name = %stem, "随包领域词库未打开，跳过");
                continue;
            }
            loaded.extend(open(&stem, &path));
        }
    }
    if let Some(dir) = user_dir {
        for (stem, path) in list(dir) {
            if !config.is_enabled(&stem) {
                tracing::debug!(name = %stem, "附加词库已关闭，跳过");
                continue;
            }
            loaded.extend(open(&stem, &path));
        }
    }
    loaded
}

fn open(stem: &str, path: &Path) -> Option<Dictionary> {
    match Dictionary::from_path(path) {
        Ok(dictionary) => {
            tracing::info!(
                name = %dictionary.metadata().map_or(stem, |m| m.name.as_str()),
                file = %path.display(),
                entries = dictionary.len(),
                license = %dictionary.metadata().map_or("", |m| m.license.as_str()),
                "附加词库已加载"
            );
            Some(dictionary)
        }
        Err(error) => {
            tracing::warn!(file = %path.display(), %error, "附加词库加载失败，跳过");
            None
        }
    }
}
