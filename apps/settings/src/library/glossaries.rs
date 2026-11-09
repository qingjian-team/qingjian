//! 释义表：哪几门语言有表（随包的或下载的），以及用户目录 `glossaries/` 里下载的那几张（「词库」页据此显示「已安装」「更新」）。
//! 找法与各平台输入法一致：下载的优先，再找随包的。

use std::path::{Path, PathBuf};

use qingjian_core::Language;
use qingjian_format::{Container, Kind as FormatKind};
use qingjian_platform::extra_dictionaries;
use qingjian_translate::{DOWNLOADED_DIR, downloaded_glossary};

use crate::i18n::{tr, tr_args};
use crate::library::Entry;
use crate::system::paths;

/// 释义表可能有的语言：英→中一张，加上各学习语言。
const LANGUAGES: [Language; 6] = [
    Language::Chinese,
    Language::English,
    Language::Japanese,
    Language::Spanish,
    Language::Vietnamese,
    Language::German,
];

/// 有释义表的语言（随包的或下载的）。
pub fn available() -> Vec<Language> {
    let downloads = paths::user_dirs().map(|dirs| dirs.downloads());
    LANGUAGES
        .into_iter()
        .filter(|language| {
            downloads
                .as_deref()
                .is_some_and(|dir| downloaded_glossary(dir, *language).is_file())
                || bundled(*language).is_some()
        })
        .collect()
}

/// 随包的释义表：macOS 在 `Resources/`，其他平台在随包根的 `data/generated/`（开发时还可能有同目录导出的 TSV）。
fn bundled(language: Language) -> Option<PathBuf> {
    let code = language.code();
    let packed = format!("glossary-{code}.qj");
    paths::mac_resources()
        .into_iter()
        .flat_map(|dir| [dir.join(&packed), dir.join(format!("glossary-{code}.tsv"))])
        .find(|path| path.is_file())
        .or_else(|| paths::bundled(&format!("data/generated/{packed}")))
        .or_else(|| paths::bundled(&format!("data/generated/glossary-{code}.tsv")))
}

/// `downloads/glossaries/` 里下载的释义表，文件名主干就是词库仓库里的 id（`glossary-ja`）。
pub fn downloaded() -> Vec<Entry> {
    let Some(dir) = paths::user_dirs().map(|dirs| dirs.downloads().join(DOWNLOADED_DIR)) else {
        return Vec::new();
    };
    extra_dictionaries::list(&dir)
        .into_iter()
        .map(|(stem, path)| entry(stem, &path))
        .collect()
}

/// 移除一张下载的释义表：挪进 `glossaries/removed/`，不真删；输入法看到文件没了就退回随包的。
pub fn remove(stem: &str) -> Result<(), String> {
    let dir = paths::user_dirs()
        .ok_or(tr("common.no_data_dir"))?
        .downloads()
        .join(DOWNLOADED_DIR);
    let (_, path) = extra_dictionaries::list(&dir)
        .into_iter()
        .find(|(name, _)| name == stem)
        .ok_or(tr("library.gone"))?;
    let removed = dir.join("removed");
    std::fs::create_dir_all(&removed)
        .map_err(|error| tr_args("library.remove_failed", &[("error", &error)]))?;
    let name = path.file_name().ok_or(tr("library.bad_name"))?;
    std::fs::rename(&path, removed.join(name))
        .map_err(|error| tr_args("library.remove_failed", &[("error", &error)]))
}

fn entry(stem: String, path: &Path) -> Entry {
    match Container::open(path, FormatKind::Glossary) {
        Ok(container) => {
            let metadata = container.metadata();
            Entry {
                name: metadata.name.clone(),
                license: metadata.license.clone(),
                version: metadata.version.clone(),
                entries: usize::try_from(metadata.entries).unwrap_or(usize::MAX),
                broken: false,
                stem,
            }
        }
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "downloaded glossary unreadable");
            Entry {
                name: stem.clone(),
                stem,
                entries: 0,
                license: String::new(),
                version: String::new(),
                broken: true,
            }
        }
    }
}
