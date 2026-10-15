//! 词库还是码表：两者的目录、配置项与列文件的方式。

use std::path::{Path, PathBuf};

use qingjian_platform::{code_tables, extra_dictionaries};

use crate::system::paths;

/// 词库还是码表。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Dictionary,
    CodeTable,
}

impl Kind {
    /// 导入、移除的结果显示在哪一行下面。
    pub fn slot(self) -> &'static str {
        match self {
            Self::Dictionary => "dicts",
            Self::CodeTable => "codes",
        }
    }

    /// 配置里记关掉了哪些的那一项。
    pub fn disabled_key(self) -> (&'static str, &'static str) {
        match self {
            Self::Dictionary => ("dictionaries", "disabled"),
            Self::CodeTable => ("aux_code", "disabled"),
        }
    }

    /// 用户导入的放在哪。
    pub fn user_dir(self) -> Option<PathBuf> {
        let name = match self {
            Self::Dictionary => "dicts",
            Self::CodeTable => "codes",
        };
        paths::user_dir().map(|dir| dir.join(name))
    }

    pub(super) fn bundled_dir(self) -> Option<PathBuf> {
        let name = match self {
            Self::Dictionary => "dicts",
            Self::CodeTable => "codes",
        };
        super::bundled_dir(name)
    }

    pub(super) fn list(self, dir: &Path) -> Vec<(String, PathBuf)> {
        match self {
            Self::Dictionary => extra_dictionaries::list(dir),
            Self::CodeTable => code_tables::list(dir),
        }
    }
}
