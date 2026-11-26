//! 把旧版本平铺在用户目录根下的文件搬进分类文件夹。目标已经有同名的就不动旧的、只记日志，不覆盖用户数据；
//! 改名失败（Windows 上 `%APPDATA%` 与 `%LOCALAPPDATA%` 不在同一个盘）就复制后删掉旧的。
//! 第一次搬之前先把用户自己产生的旧文件原样复制一份进 `backup-before-0.3.4/`。

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

/// 搬之前的备份：保持旧版本的平铺结构，退回旧版本时把里面的东西拷回根目录就能用。不自动删，留给用户确认没问题后自己删。
const BACKUP_DIR: &str = "backup-before-0.3.4";

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
    let downloaded = downloaded_ids(dirs);
    backup(dirs, &names, &downloaded);
    for name in names {
        let source = root.join(&name);
        if let Some(target) = target_of(dirs, &name, source.is_dir()) {
            move_path(&source, &target);
        }
    }
    // 下载的与导入的附加词库原来都在 `dicts/`：词库列表里有的 id 是下载的，其余是导入的
    let legacy_dicts = root.join("dicts");
    if legacy_dicts.is_dir() {
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

/// 把要搬的、用户自己产生的旧文件复制进 [`BACKUP_DIR`]：模型、下载的词库与缓存能重新下载，不备份。
/// 备份文件夹已经在、或没有要搬的就不做；先复制到本进程自己的临时名，完整了再改名，
/// 输入法与设置程序同时启动时只留一份。复制失败只记日志、照样搬：搬本身不覆盖也不删用户数据。
fn backup(dirs: &UserDirs, names: &[String], downloaded: &HashSet<String>) {
    let root = dirs.root();
    let backup = root.join(BACKUP_DIR);
    if backup.exists() {
        return;
    }
    let mut items: Vec<PathBuf> = names
        .iter()
        .filter(|name| {
            target_of(dirs, name, root.join(name).is_dir()).is_some_and(|target| {
                !target.starts_with(dirs.downloads()) && !target.starts_with(dirs.cache())
            })
        })
        .map(PathBuf::from)
        .collect();
    if let Ok(entries) = std::fs::read_dir(root.join("dicts")) {
        items.extend(
            entries
                .filter_map(Result::ok)
                .filter(|entry| !is_downloaded_dict(&entry.path(), downloaded))
                .map(|entry| Path::new("dicts").join(entry.file_name())),
        );
    }
    if items.is_empty() {
        return;
    }
    let staging = root.join(format!(".{BACKUP_DIR}.{}.partial", std::process::id()));
    let _ = remove_all(&staging);
    let copied = items
        .iter()
        .try_for_each(|item| {
            let target = staging.join(item);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            copy_all(&root.join(item), &target)
        })
        .and_then(|()| std::fs::rename(&staging, &backup));
    match copied {
        Ok(()) => tracing::info!(
            dir = %backup.display(),
            items = items.len(),
            "user data backed up before moving"
        ),
        Err(error) => {
            let _ = remove_all(&staging);
            // 另一个进程先备份完、已经开始搬了
            if !backup.exists() {
                tracing::warn!(%error, "user data not backed up");
            }
        }
    }
}

/// 旧 `dicts/` 里的一项是不是从词库仓库下载的：文件名（不带扩展名）在词库列表里。
fn is_downloaded_dict(path: &Path, downloaded: &HashSet<String>) -> bool {
    path.is_file()
        && path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| downloaded.contains(stem))
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
        let target = if is_downloaded_dict(&source, downloaded) {
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
