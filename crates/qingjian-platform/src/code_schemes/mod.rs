//! 形码方案：随包的（如 86 五笔）与用户数据目录 `schemes/<id>/` 下自己放进去的，同一种格式、同一套找法。
//!
//! 一套方案一个目录，目录名就是 id：`scheme.toml` 写名字、元信息与按键规则（见 `Descriptor`，必须有），码表是 `词\t编码\t词频` 的 TSV。
//! 用户目录里与随包同 id 的方案盖过随包那套；老位置 `wubi/wubi86.tsv` 仍认，只换随包五笔的码表。
//! 用户导入的形码词库在用户目录 `code_dicts/` 下（见 [`qingjian_dictionary::import::import_code_words`]），按文件头记的方案 id
//! 跟那套方案的码表合在一起，不用整份换掉码表。
//! 随包的 `schemes` 目录在哪由壳给（macOS 在 `Resources/`，Windows 在安装目录的 `assets/`）。

mod descriptor;
mod entry;

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use qingjian_dictionary::import::code_words_scheme;

use self::descriptor::Descriptor;
pub use self::entry::CodeSchemeEntry;
use crate::Scheme;

/// `scheme.toml` 里 `type` 写这个（或不写）就是形码方案。
pub const KIND: &str = "code";

/// 随包的 86 五笔。
pub const WUBI86: &str = "wubi86";

/// 方案目录名，随包与用户数据目录下都叫这个。
pub const DIR: &str = "schemes";

pub(crate) const DESCRIPTOR: &str = "scheme.toml";

/// 导入的形码词库的目录名，在用户数据目录下。
const WORDS_DIR: &str = "code_dicts";

/// 老位置：用户自己换的五笔码表（2026-09 起就认这里）。
const LEGACY_WUBI_TABLE: &str = "wubi/wubi86.tsv";

/// 能当方案 id 的写法：小写字母、数字、`-` `_`，而且不能与拼音侧的写法或日志里 86 五笔的 `wubi` 撞上——
/// 输入日志把两侧拼成 `xiaohe+<id>`，撞了回放就认错方案。`wubi86` 例外：它在拼音侧是老配置的写法，
/// 在这里就是随包五笔本身（用户目录里放 `schemes/wubi86/` 是换掉随包那套）。
pub fn is_valid_id(id: &str) -> bool {
    if id == WUBI86 {
        return true;
    }
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_'))
        && id != "wubi"
        && id.parse::<Scheme>().is_err()
}

/// 全部可用的方案：随包的在前，用户的在后，各自按 id 排；用户方案与随包同 id 时占随包那个位置。
/// `bundled` 是随包的 `schemes` 目录。
pub fn list(user_dir: Option<&Path>, bundled: Option<&Path>) -> Vec<CodeSchemeEntry> {
    let mut entries = bundled.map_or_else(Vec::new, |dir| scan(dir, true));
    if let Some(user_dir) = user_dir {
        let legacy = user_dir.join(LEGACY_WUBI_TABLE);
        if legacy.is_file()
            && let Some(wubi) = entries.iter_mut().find(|entry| entry.id == WUBI86)
        {
            wubi.table = legacy;
        }
        for entry in scan(&user_dir.join(DIR), false) {
            match entries.iter_mut().find(|bundled| bundled.id == entry.id) {
                Some(slot) => *slot = entry,
                None => entries.push(entry),
            }
        }
        for words in list_words(&user_words_dir(user_dir)) {
            if let Some(entry) = entries.iter_mut().find(|entry| entry.id == words.scheme) {
                entry.words.push(words.path);
            }
        }
    }
    entries
}

/// 用户导入的一本形码词库。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeWords {
    /// 文件名（不含扩展名），`[dictionaries] disabled` 按它开关。
    pub stem: String,

    pub path: PathBuf,

    /// 给哪套方案的。
    pub scheme: String,
}

/// 形码词库目录（`dir`，见 [`user_words_dir`]）的逐文件快照（路径、mtime、长度）：壳每秒比一次，设置页刚导入、移除就重装方案，不等配置改动。
pub fn words_snapshot(dir: &Path) -> Vec<(PathBuf, Option<SystemTime>, u64)> {
    word_files(dir)
        .into_iter()
        .filter_map(|path| {
            let metadata = std::fs::metadata(&path).ok()?;
            Some((path, metadata.modified().ok(), metadata.len()))
        })
        .collect()
}

/// 形码词库目录里的词库，按文件名排。
pub fn list_words(dir: &Path) -> Vec<CodeWords> {
    word_files(dir)
        .into_iter()
        .filter_map(|path| {
            Some(CodeWords {
                stem: path.file_stem()?.to_str()?.to_owned(),
                scheme: code_words_scheme(&path)?,
                path,
            })
        })
        .collect()
}

/// 目录里的 `.tsv`，按路径排；以 `.` 开头的（访达拷出来的 `._*`）跳过。
fn word_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(children) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = children
        .filter_map(Result::ok)
        .map(|child| child.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| !name.starts_with('.') && name.ends_with(".tsv"))
        })
        .collect();
    files.sort();
    files
}

/// 按 id 找一套方案；找不到为 `None`。
pub fn find(id: &str, user_dir: Option<&Path>, bundled: Option<&Path>) -> Option<CodeSchemeEntry> {
    list(user_dir, bundled)
        .into_iter()
        .find(|entry| entry.id == id)
}

/// 用户方案的目录：`<用户数据目录>/schemes`，不负责创建。
pub fn user_schemes_dir(user_dir: &Path) -> PathBuf {
    user_dir.join(DIR)
}

/// 导入的形码词库的目录：`<用户数据目录>/code_dicts`，不负责创建。
pub fn user_words_dir(user_dir: &Path) -> PathBuf {
    user_dir.join(WORDS_DIR)
}

/// 一个 `schemes` 目录下的方案，按 id 排。不能用的跳过并记日志：一套方案坏了不能拖垮别的。
fn scan(dir: &Path, bundled: bool) -> Vec<CodeSchemeEntry> {
    let Ok(children) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut entries: Vec<CodeSchemeEntry> = children
        .filter_map(Result::ok)
        .map(|child| child.path())
        .filter(|path| path.is_dir())
        .filter_map(|path| read(&path, bundled))
        .collect();
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    entries
}

/// 读一个用户方案目录（目录名就是 id）；不能用时记日志、返回 `None`。
pub fn from_dir(dir: &Path) -> Option<CodeSchemeEntry> {
    read(dir, false)
}

fn read(dir: &Path, bundled: bool) -> Option<CodeSchemeEntry> {
    let id = dir.file_name()?.to_str()?.to_owned();
    if !is_valid_id(&id) {
        tracing::warn!(dir = %dir.display(), "形码方案的目录名不能当 id，跳过");
        return None;
    }
    let path = dir.join(DESCRIPTOR);
    let descriptor: Descriptor = match std::fs::read_to_string(&path) {
        Ok(text) => match toml::from_str(&text) {
            Ok(descriptor) => descriptor,
            Err(error) => {
                tracing::warn!(file = %path.display(), %error, "scheme.toml 解析失败，跳过");
                return None;
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            tracing::warn!(dir = %dir.display(), "方案目录里没有 scheme.toml，跳过");
            return None;
        }
        Err(error) => {
            tracing::warn!(file = %path.display(), %error, "scheme.toml 读不了，跳过");
            return None;
        }
    };
    match descriptor.kind.as_str() {
        "" | KIND => {}
        crate::shuangpin_schemes::KIND | crate::aux_schemes::KIND => return None,
        kind => {
            tracing::warn!(file = %path.display(), kind, "不认识的方案种类，跳过");
            return None;
        }
    }
    let table = dir.join(&descriptor.table);
    if !table.is_file() {
        tracing::warn!(file = %table.display(), "形码方案的码表不在，跳过");
        return None;
    }
    let label = match descriptor.name.trim() {
        "" => id.clone(),
        name => name.to_owned(),
    };
    Some(CodeSchemeEntry {
        id,
        label,
        meta: descriptor.meta,
        table,
        words: Vec::new(),
        auto_commit: descriptor.auto_commit,
        top: descriptor.top,
        code_length: descriptor.code_length,
        bundled,
    })
}

#[cfg(test)]
mod tests;
