//! 从词库仓库下载的释义表：设置页装进用户目录的 `glossaries/`，各平台壳先找这里、再找随包的，
//! 所以下载的新版能盖过随包的旧版。

use std::path::{Path, PathBuf};

use qingjian_core::Language;

/// 用户目录下放下载释义表的子目录。
pub const DOWNLOADED_DIR: &str = "glossaries";

/// 某语言下载释义表的位置（不管文件在不在）：`<用户目录>/glossaries/glossary-<语言>.qj`。
pub fn downloaded_glossary(user_dir: &Path, language: Language) -> PathBuf {
    user_dir
        .join(DOWNLOADED_DIR)
        .join(format!("glossary-{}.qj", language.code()))
}
