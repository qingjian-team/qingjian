//! 附加词库：随包的领域词库（`Resources/dicts/`，按 `[dictionaries] domains` 挑）与用户目录 `dicts/` 下的 `.qj`（或 TSV）
//! 文件（按 `[dictionaries] disabled` 过滤），加载后一起接到 Engine 上。

use std::path::{Path, PathBuf};

use crate::DictionariesConfig;
use qingjian_dictionary::Dictionary;

/// 目录里能加载的扩展名，靠前的优先：同名的 `.qj` 与 `.tsv` 只取 `.qj`（开发目录里两者并存）。
const EXTENSIONS: [&str; 2] = ["qj", "tsv"];

/// 列出目录里的词库文件（按文件名排序，同名只留优先扩展名的那个），返回 (文件名不含扩展名, 路径)。目录不存在就是空。
pub fn list(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<(String, usize, PathBuf)> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter_map(|p| {
            let extension = p.extension()?.to_str()?;
            let rank = EXTENSIONS.iter().position(|e| *e == extension)?;
            let stem = p.file_stem()?.to_str()?.to_owned();
            Some((stem, rank, p))
        })
        .collect();
    files.sort();
    files.dedup_by(|a, b| a.0 == b.0);
    files
        .into_iter()
        .map(|(stem, _, path)| (stem, path))
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_prefers_packed_over_tsv_with_same_stem() {
        let dir = std::env::temp_dir().join(format!("qingjian-extra-dicts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["idioms.qj", "idioms.tsv", "food.tsv", "notes.txt"] {
            std::fs::write(dir.join(name), b"").unwrap();
        }

        let listed = list(&dir);
        let names: Vec<(&str, &str)> = listed
            .iter()
            .map(|(stem, path)| (stem.as_str(), path.file_name().unwrap().to_str().unwrap()))
            .collect();
        assert_eq!(names, [("food", "food.tsv"), ("idioms", "idioms.qj")]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
