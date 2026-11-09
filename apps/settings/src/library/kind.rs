//! 词库、形码词库还是码表：各自的目录、配置项与列文件的方式。

use std::path::{Path, PathBuf};

use qingjian_platform::{code_schemes, code_tables, extra_dictionaries};

use crate::system::paths;

/// 词库、形码词库还是码表。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Dictionary,

    /// 导入的五笔这类词库（`code_dicts/`）：和拼音词库同一个导入入口、同一组开关，载入时并进形码方案的码表。
    CodeWords,

    CodeTable,
}

impl Kind {
    /// 导入、移除的结果显示在哪一行下面。
    pub fn slot(self) -> &'static str {
        match self {
            Self::Dictionary | Self::CodeWords => "dicts",
            Self::CodeTable => "codes",
        }
    }

    /// 配置里记关掉了哪些的那一项。
    pub fn disabled_key(self) -> (&'static str, &'static str) {
        match self {
            Self::Dictionary | Self::CodeWords => ("dictionaries", "disabled"),
            Self::CodeTable => ("aux_code", "disabled"),
        }
    }

    /// 用户导入的放在哪。
    pub fn user_dir(self) -> Option<PathBuf> {
        let user = paths::user_dir()?;
        Some(match self {
            Self::Dictionary => user.join("dicts"),
            Self::CodeWords => code_schemes::user_words_dir(&user),
            Self::CodeTable => user.join("codes"),
        })
    }

    /// 随包的那一份；形码词库只有用户导入的。
    pub(super) fn bundled_dir(self) -> Option<PathBuf> {
        match self {
            Self::Dictionary => super::bundled_dir("dicts"),
            Self::CodeWords => None,
            Self::CodeTable => super::bundled_dir("codes"),
        }
    }

    pub(super) fn list(self, dir: &Path) -> Vec<(String, PathBuf)> {
        match self {
            Self::Dictionary => extra_dictionaries::list(dir),
            Self::CodeWords => code_schemes::list_words(dir)
                .into_iter()
                .map(|words| (words.stem, words.path))
                .collect(),
            Self::CodeTable => code_tables::list(dir),
        }
    }
}
