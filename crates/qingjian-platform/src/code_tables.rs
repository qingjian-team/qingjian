//! 辅码码表：随包的（随包根 `codes/`）与用户目录 `codes/` 下的 `.qj`，按 `[aux_code] disabled` 过滤，
//! 加载后一起接到 Engine 上。与 `extra_dictionaries` 同构，只是装的是码表。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use qingjian_dictionary::{AuxCodeLookup, AuxCodeTable};

use crate::AuxCodeConfig;

/// 码表的扩展名：只认打包过的 `.qj`（TSV 在导入时就转掉了）。
const EXTENSION: &str = "qj";

/// 列出目录里的码表文件（按文件名排序），返回 (文件名不含扩展名, 路径)。目录不存在就是空。
pub fn list(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<(String, PathBuf)> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter_map(|path| {
            (path.extension()?.to_str()? == EXTENSION).then(|| {
                let stem = path.file_stem()?.to_str()?.to_owned();
                Some((stem, path))
            })?
        })
        .collect();
    files.sort();
    files
}

/// 可加载码表的快照：用于发现新增、移除与同名更新，不读取码表正文。
/// 逐文件比 (mtime, 长度)：半拷进来的 `.qj`（写完长度才稳定）与就地改写都能被发现。
pub fn snapshot(dir: &Path) -> Vec<(PathBuf, Option<std::time::SystemTime>, u64)> {
    list(dir)
        .into_iter()
        .filter_map(|(_, path)| {
            let metadata = std::fs::metadata(&path).ok()?;
            Some((path, metadata.modified().ok(), metadata.len()))
        })
        .collect()
}

/// 加载没被关掉的码表：先随包，再用户目录。坏文件只记日志、跳过：一张码表坏了不能拖垮输入法。
pub fn load(
    bundled_dir: Option<&Path>,
    user_dir: Option<&Path>,
    config: &AuxCodeConfig,
) -> Vec<Arc<dyn AuxCodeLookup>> {
    let mut loaded: Vec<Arc<dyn AuxCodeLookup>> = Vec::new();
    for dir in [bundled_dir, user_dir].into_iter().flatten() {
        for (stem, path) in list(dir) {
            if !config.is_enabled(&stem) {
                tracing::debug!(name = %stem, "码表已关闭，跳过");
                continue;
            }
            match AuxCodeTable::open(&path) {
                Ok(table) => {
                    tracing::info!(
                        name = %table.metadata().map_or(stem.as_str(), |m| m.name.as_str()),
                        file = %path.display(),
                        entries = table.len(),
                        words = table.word_count(),
                        license = %table.metadata().map_or("", |m| m.license.as_str()),
                        "码表已加载"
                    );
                    loaded.push(Arc::new(table));
                }
                Err(error) => tracing::warn!(file = %path.display(), %error, "码表加载失败，跳过"),
            }
        }
    }
    loaded
}
