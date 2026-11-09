//! 英文音标表（随包 `english-ipa.qj`，词库仓库发的那本）的加载，各平台装配共用。
//!
//! 音标表是可选的：没有或坏了只是英文候选不显示音标，不影响候选本身，所以失败只记日志。

use std::path::Path;

use qingjian_dictionary::WordList;

/// 文件名：随包放在 `data/generated/`（macOS 在 `Resources/`）。
pub const ENGLISH_PRONUNCIATIONS_FILE: &str = "english-ipa.qj";

/// 读音标表，`.qj` 与开发时的 TSV（`词\t/音标/`）都认；读不了返回 `None`。
pub fn load_english_pronunciations(path: &Path) -> Option<WordList> {
    match WordList::pronunciations_from_path(path) {
        Ok(table) => {
            tracing::info!(words = table.len(), "english pronunciations loaded");
            Some(table)
        }
        Err(error) => {
            tracing::warn!(%error, path = %path.display(), "english pronunciations failed to load");
            None
        }
    }
}
