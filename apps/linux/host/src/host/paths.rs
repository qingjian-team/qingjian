//! 路径约定(XDG)与数据文件探测、释义表装载。

use std::path::{Path, PathBuf};

use qingjian_core::Language;
use qingjian_translate::Glossary;

/// 配置文件：$XDG_CONFIG_HOME/qingjian/config.toml，缺省 ~/.config/qingjian/config.toml。
pub fn config_path() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME")
        && !dir.is_empty()
    {
        return Some(PathBuf::from(dir).join("qingjian/config.toml"));
    }
    std::env::var("HOME")
        .ok()
        .map(|home| PathBuf::from(home).join(".config/qingjian/config.toml"))
}

/// 数据目录：$XDG_DATA_HOME/qingjian，缺省 ~/.local/share/qingjian。
pub fn data_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("XDG_DATA_HOME")
        && !dir.is_empty()
    {
        return Some(PathBuf::from(dir).join("qingjian"));
    }
    std::env::var("HOME")
        .ok()
        .map(|home| PathBuf::from(home).join(".local/share/qingjian"))
}

pub(super) fn mtime_of(path: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// 按配置的学习语言挑释义表并装载；没有对应文件依次退回英语、任一存在的。
pub(super) fn load_glossary(dir: &Path, configured: &str) -> Option<(Language, Glossary)> {
    let configured = configured.parse::<Language>().ok();
    let language = [configured, Some(Language::English)]
        .into_iter()
        .flatten()
        .find(|l| find_data(dir, &format!("glossary-{}", l.code())).is_some())?;
    let path = find_data(dir, &format!("glossary-{}", language.code()))?;
    match Glossary::from_path(language, &path) {
        Ok(glossary) => {
            tracing::info!(
                language = language.code(),
                glosses = glossary.len(),
                "释义表已加载"
            );
            Some((language, glossary))
        }
        Err(error) => {
            tracing::warn!(%error, "释义表加载失败,候选无译文");
            None
        }
    }
}

/// 数据查找分两层（fcitx5 StandardPaths 的「用户层盖过系统层」同款语义，见 install.sh）：
/// 数据目录根 = 用户层（学习数据 + 用户自有覆盖件，安装器不碰），
/// `dist/` = 随包层（安装器独占，每次安装整个换新，升级即更新）。
pub(super) fn data_layers(dir: &Path) -> [PathBuf; 2] {
    [dir.to_path_buf(), dir.join("dist")]
}

pub(super) fn find_data(dir: &Path, stem: &str) -> Option<PathBuf> {
    // 用户层整层先于随包层；层内同名 .qj 优先于 .tsv，与 extra_dictionaries 的约定一致。
    for base in data_layers(dir) {
        for ext in ["qj", "tsv"] {
            let path = base.join(format!("{stem}.{ext}"));
            if path.is_file() {
                return Some(path);
            }
        }
    }
    None
}

/// 精确文件名版（emoji 表、词汇等级表这类没有 .qj 变体的）：同样用户层先于随包层。
pub(super) fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    data_layers(dir)
        .into_iter()
        .map(|base| base.join(name))
        .find(|path| path.is_file())
}
