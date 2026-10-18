//! 词库与码表的清单：随包的、用户导入的各有哪些，名字、条数、许可证。进词库页（或导入、移除）时扫一次，不每帧读盘。
//! 移除是挪进目录里的 `removed/`，不真删。

mod entry;
mod kind;

pub use entry::Entry;
pub use kind::Kind;

use std::path::{Path, PathBuf};

use qingjian_dictionary::{Dictionary, aux_code_table_info};

use crate::system::paths;

#[derive(Debug, Clone, Default)]
pub struct Library {
    pub domains: Vec<Entry>,

    pub dictionaries: Vec<Entry>,

    pub bundled_codes: Vec<Entry>,

    pub codes: Vec<Entry>,
}

impl Library {
    pub fn scan() -> Self {
        let scan = |kind: Kind, dir: Option<PathBuf>| -> Vec<Entry> {
            dir.map(|dir| {
                kind.list(&dir)
                    .into_iter()
                    .map(|(stem, path)| entry(kind, stem, &path))
                    .collect()
            })
            .unwrap_or_default()
        };
        Self {
            domains: scan(Kind::Dictionary, Kind::Dictionary.bundled_dir()),
            dictionaries: scan(Kind::Dictionary, Kind::Dictionary.user_dir()),
            bundled_codes: scan(Kind::CodeTable, Kind::CodeTable.bundled_dir()),
            codes: scan(Kind::CodeTable, Kind::CodeTable.user_dir()),
        }
    }
}

fn entry(kind: Kind, stem: String, path: &Path) -> Entry {
    match kind {
        Kind::Dictionary => match Dictionary::from_path(path) {
            Ok(dictionary) => {
                let metadata = dictionary.metadata();
                Entry {
                    name: metadata.map_or_else(|| stem.clone(), |m| m.name.clone()),
                    license: metadata.map_or_else(String::new, |m| m.license.clone()),
                    entries: dictionary.len(),
                    broken: false,
                    stem,
                }
            }
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "词库打不开");
                Entry {
                    name: stem.clone(),
                    stem,
                    entries: 0,
                    license: String::new(),
                    broken: true,
                }
            }
        },
        Kind::CodeTable => {
            let info = aux_code_table_info(path);
            Entry {
                stem,
                name: info.name,
                entries: info.entries,
                license: info.license,
                broken: info.broken,
            }
        }
    }
}

/// 随包的 `dicts` / `codes` 目录，与各平台输入法读的同一处：
/// macOS 在 `Qingjian.app/Contents/Resources/`（设置程序将来也装在包里，先找自己所在的包，再找装好的输入法），
/// 其他平台在随包根的 `data/generated/`；都找不到时用开发布局（仓库的 `data/generated/`）。
fn bundled_dir(name: &str) -> Option<PathBuf> {
    paths::mac_resources()
        .into_iter()
        .map(|dir| dir.join(name))
        .find(|dir| dir.is_dir())
        .or_else(|| paths::bundled(&format!("data/generated/{name}")))
}

/// 挪进 `removed/`；输入法看到文件没了会自己卸掉。
pub fn remove(kind: Kind, stem: &str) -> Result<(), String> {
    let dir = kind.user_dir().ok_or("找不到数据目录")?;
    let (_, path) = kind
        .list(&dir)
        .into_iter()
        .find(|(name, _)| name == stem)
        .ok_or("文件已经不在了")?;
    let removed = dir.join("removed");
    std::fs::create_dir_all(&removed).map_err(|error| format!("移除失败：{error}"))?;
    let name = path.file_name().ok_or("文件名不对")?;
    std::fs::rename(&path, removed.join(name)).map_err(|error| format!("移除失败：{error}"))
}
