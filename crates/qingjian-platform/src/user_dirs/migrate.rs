//! 把旧版本平铺在用户目录根下的文件搬进分类文件夹。目标已经有同名的就不动旧的、只记日志，不覆盖用户数据；
//! 改名失败（Windows 上 `%APPDATA%` 与 `%LOCALAPPDATA%` 不在同一个盘）就复制后删掉旧的。

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::UserDirs;

/// 记录类的文件：生词本 `user-vocab.tsv` 也是 `user` 开头，但它是记录，不是学习数据，要先于学习数据认。
const RECORD_FILES: [&str; 9] = [
    "user-vocab.tsv",
    "vocab-log.tsv",
    "usage-hourly.tsv",
    "usage.tsv",
    "panel-usage.tsv",
    "api-usage.json",
    "api-usage-hourly.tsv",
    "api-usage.lock",
    "input-log.jsonl",
];

const CACHE_FILES: [&str; 2] = ["dict-index.json", "update.json"];

/// 原来就是文件夹、整个挪到 `custom/` 下的。
const CUSTOM_DIRS: [&str; 4] = ["code_dicts", "codes", "schemes", "themes"];

/// 原来就是文件夹、整个挪到 `downloads/` 下的。
const DOWNLOAD_DIRS: [&str; 2] = ["glossaries", "models"];

pub(super) fn run(dirs: &UserDirs) {
    let root = dirs.root();
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    names.sort();
    for name in names {
        let source = root.join(&name);
        if let Some(target) = target_of(dirs, &name, source.is_dir()) {
            move_path(&source, &target);
        }
    }
    // 下载的与导入的附加词库原来都在 `dicts/`：词库列表里有的 id 是下载的，其余是导入的
    let legacy_dicts = root.join("dicts");
    if legacy_dicts.is_dir() {
        let downloaded = downloaded_ids(dirs);
        split_dicts(&legacy_dicts, dirs, &downloaded);
    }
}

/// 根目录下一项该搬去哪；`dicts/` 要按文件拆开，另外处理；配置、剪贴板和认不出的都留在根下。
fn target_of(dirs: &UserDirs, name: &str, is_dir: bool) -> Option<PathBuf> {
    if is_dir {
        // 更早的版本把通变放在根下的 `model-p2c/`
        if name == "model-p2c" {
            return Some(dirs.models().join("hanzhang-tongbian"));
        }
        if CUSTOM_DIRS.contains(&name) {
            return Some(dirs.custom().join(name));
        }
        if DOWNLOAD_DIRS.contains(&name) {
            return Some(dirs.downloads().join(name));
        }
        return None;
    }
    if RECORD_FILES.contains(&name) {
        return Some(dirs.records().join(name));
    }
    if CACHE_FILES.contains(&name) {
        return Some(dirs.cache().join(name));
    }
    // 词频、造词、选词、降权、n-gram、敲错表、英文词与个人释义表都是 `user*.tsv`，连同它们的 `.bak`
    (name.starts_with("user") && name.contains(".tsv")).then(|| dirs.learning().join(name))
}

/// 词库列表缓存里的 id：旧位置在根下，已经搬过就在 `cache/` 里。
fn downloaded_ids(dirs: &UserDirs) -> HashSet<String> {
    [
        dirs.root().join("dict-index.json"),
        dirs.cache().join("dict-index.json"),
    ]
    .iter()
    .filter_map(|path| std::fs::read(path).ok())
    .filter_map(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
    .flat_map(|index| {
        ["dicts", "bundled"]
            .into_iter()
            .filter_map(|key| index.get(key)?.as_array().cloned())
            .flatten()
            .filter_map(|entry| Some(entry.get("id")?.as_str()?.to_owned()))
            .collect::<Vec<_>>()
    })
    .collect()
}

fn split_dicts(legacy: &Path, dirs: &UserDirs, downloaded: &HashSet<String>) {
    let Ok(entries) = std::fs::read_dir(legacy) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let source = entry.path();
        let name = entry.file_name();
        let is_download = source.is_file()
            && source
                .file_stem()
                .and_then(|stem| stem.to_str())
                .is_some_and(|stem| downloaded.contains(stem));
        let target = if is_download {
            dirs.downloaded_dicts().join(&name)
        } else {
            dirs.custom_dicts().join(&name)
        };
        move_path(&source, &target);
    }
    // 拆空了才删；留下的是目标已有同名、没搬走的
    let _ = std::fs::remove_dir(legacy);
}

/// 搬一个文件或文件夹。文件夹的目标已经存在就逐项合并进去。
fn move_path(source: &Path, target: &Path) {
    if source.is_dir() && target.is_dir() {
        if let Ok(entries) = std::fs::read_dir(source) {
            for entry in entries.filter_map(Result::ok) {
                move_path(&entry.path(), &target.join(entry.file_name()));
            }
        }
        let _ = std::fs::remove_dir(source);
        return;
    }
    if target.exists() {
        tracing::warn!(
            source = %source.display(),
            target = %target.display(),
            "user data not moved: target already exists"
        );
        return;
    }
    if let Some(parent) = target.parent()
        && let Err(error) = std::fs::create_dir_all(parent)
    {
        tracing::warn!(dir = %parent.display(), %error, "user data folder not created");
        return;
    }
    let moved = std::fs::rename(source, target).or_else(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            return Err(error);
        }
        copy_across(source, target)?;
        remove_all(source)
    });
    match moved {
        Ok(()) => tracing::info!(
            source = %source.display(),
            target = %target.display(),
            "user data moved"
        ),
        // 另一个进程刚搬走了
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => tracing::warn!(
            source = %source.display(),
            target = %target.display(),
            %error,
            "user data not moved"
        ),
    }
}

/// 跨盘搬：先复制到旁边的临时名，完整了再改名，中途失败不会留下半份让下次以为已经搬过。
fn copy_across(source: &Path, target: &Path) -> std::io::Result<()> {
    let name = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("item");
    let staging = target.with_file_name(format!(".{name}.moving"));
    let _ = remove_all(&staging);
    let copied = copy_all(source, &staging).and_then(|()| std::fs::rename(&staging, target));
    if copied.is_err() {
        let _ = remove_all(&staging);
    }
    copied
}

fn copy_all(source: &Path, target: &Path) -> std::io::Result<()> {
    if source.is_dir() {
        std::fs::create_dir_all(target)?;
        for entry in std::fs::read_dir(source)? {
            let entry = entry?;
            copy_all(&entry.path(), &target.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        std::fs::copy(source, target).map(|_| ())
    }
}

fn remove_all(path: &Path) -> std::io::Result<()> {
    if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
}
