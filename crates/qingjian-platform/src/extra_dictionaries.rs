//! 附加词库：随包的领域词库（`Resources/dicts/`，按 `[dictionaries] domains` 挑）与用户目录里导入的、下载的 `.qj`（或 TSV）
//! 文件（`custom/dicts/` 与 `downloads/dicts/`，按 `[dictionaries] disabled` 过滤），加载后一起接到 Engine 上。
//! 通用大词库（[`GENERAL_DICTIONARY`]）不管在哪都标成补充词库（排序低主词库一档，见 `Dictionary::set_secondary`）；随包的那本缺省打开、列进 `disabled` 才关。

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::DictionariesConfig;
use qingjian_dictionary::{Dictionary, GENERAL_DICTIONARY};

/// 目录里能加载的扩展名，靠前的优先：同名的 `.qj` 与 `.tsv` 只取 `.qj`（开发目录里两者并存）。
const EXTENSIONS: [&str; 2] = ["qj", "tsv"];

/// 可加载文件的快照：用于发现新增、移除与同名更新，不读取词库正文。
pub fn snapshot(dir: &Path) -> Vec<(PathBuf, Option<SystemTime>, u64)> {
    list(dir)
        .into_iter()
        .filter_map(|(_, path)| {
            let metadata = std::fs::metadata(&path).ok()?;
            Some((path, metadata.modified().ok(), metadata.len()))
        })
        .collect()
}

/// 列出目录里的词库文件（按文件名排序，同名只留优先扩展名的那个），返回 (文件名不含扩展名, 路径)。目录不存在就是空。
/// 以 `.` 开头的隐藏文件跳过：macOS 打包 / 拷贝带出的 `._*` 元数据不是词库。
pub fn list(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<(String, usize, PathBuf)> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter_map(|p| {
            let name = p.file_name()?.to_str()?;
            if name.starts_with('.') {
                return None;
            }
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

/// 加载没被关掉的词库：先随包领域词库，再用户目录（导入的、下载的）。坏文件只记日志、跳过：一本词库坏了不能拖垮输入法。
/// 随包里有通用大词库时，用户目录里同名的那本（从下载列表装的）不再加载，免得同一批词挂两份。
pub fn load(
    bundled_dir: Option<&Path>,
    user_dirs: &[&Path],
    config: &DictionariesConfig,
) -> Vec<Dictionary> {
    let mut loaded = Vec::new();
    let mut bundled_general = false;
    if let Some(dir) = bundled_dir {
        for (stem, path) in list(dir) {
            if stem == GENERAL_DICTIONARY {
                bundled_general = true;
                if config.is_enabled(&stem) {
                    loaded.extend(open(&stem, &path));
                }
                continue;
            }
            if !config.is_domain_enabled(&stem) {
                tracing::debug!(name = %stem, "bundled domain dictionary off, skipped");
                continue;
            }
            loaded.extend(open(&stem, &path));
        }
    }
    for dir in user_dirs {
        for (stem, path) in list(dir) {
            if !config.is_enabled(&stem) {
                tracing::debug!(name = %stem, "extra dictionary disabled, skipped");
                continue;
            }
            if bundled_general && stem == GENERAL_DICTIONARY {
                tracing::debug!(file = %path.display(), "general dictionary is bundled, user copy skipped");
                continue;
            }
            loaded.extend(open(&stem, &path));
        }
    }
    loaded
}

fn open(stem: &str, path: &Path) -> Option<Dictionary> {
    match Dictionary::from_path(path) {
        Ok(mut dictionary) => {
            dictionary.set_secondary(stem == GENERAL_DICTIONARY);
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

    #[test]
    fn list_skips_hidden_files() {
        let dir = std::env::temp_dir().join(format!("qingjian-dot-dicts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["._idioms.qj", ".hidden.qj", "idioms.qj"] {
            std::fs::write(dir.join(name), b"").unwrap();
        }

        let listed = list(&dir);
        let names: Vec<(&str, &str)> = listed
            .iter()
            .map(|(stem, path)| (stem.as_str(), path.file_name().unwrap().to_str().unwrap()))
            .collect();
        assert_eq!(names, [("idioms", "idioms.qj")]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 通用大词库在用户目录里（设置里下载的）时也按补充词库排在主词库后面；别的用户词库照常。
    #[test]
    fn general_dictionary_in_user_dir_stays_secondary() {
        let dir =
            std::env::temp_dir().join(format!("qingjian-user-general-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(format!("{GENERAL_DICTIONARY}.tsv")),
            "按照规定\tan zhao gui ding\t10\n",
        )
        .unwrap();
        std::fs::write(dir.join("law.tsv"), "合同法\the tong fa\t10\n").unwrap();

        let loaded = load(None, &[dir.as_path()], &DictionariesConfig::default());
        let secondary: Vec<bool> = loaded.iter().map(Dictionary::is_secondary).collect();
        assert_eq!(secondary, [true, false]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 随包和用户目录里都有通用大词库时只挂随包那本；随包的被关掉时用户那本也不挂（同一个开关）。
    #[test]
    fn bundled_general_dictionary_shadows_user_copy() {
        let root =
            std::env::temp_dir().join(format!("qingjian-bundled-general-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (bundled, user) = (root.join("bundled"), root.join("user"));
        for dir in [&bundled, &user] {
            std::fs::create_dir_all(dir).unwrap();
            std::fs::write(
                dir.join(format!("{GENERAL_DICTIONARY}.tsv")),
                "按照规定\tan zhao gui ding\t10\n",
            )
            .unwrap();
        }
        std::fs::write(user.join("law.tsv"), "合同法\the tong fa\t10\n").unwrap();

        let loaded = load(
            Some(&bundled),
            &[user.as_path()],
            &DictionariesConfig::default(),
        );
        let secondary: Vec<bool> = loaded.iter().map(Dictionary::is_secondary).collect();
        assert_eq!(secondary, [true, false]);

        let off = DictionariesConfig {
            disabled: vec![GENERAL_DICTIONARY.to_owned()],
            ..DictionariesConfig::default()
        };
        let loaded = load(Some(&bundled), &[user.as_path()], &off);
        let secondary: Vec<bool> = loaded.iter().map(Dictionary::is_secondary).collect();
        assert_eq!(secondary, [false]);
        let _ = std::fs::remove_dir_all(&root);
    }
}
