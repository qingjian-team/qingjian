//! 一次配置读取里被丢掉的分节与条目。读配置不因为一条坏规则全盘失效，丢掉的记在这里给菜单、偏好设置与日志看。

use std::fmt;

/// 丢弃记录：每条一句话，说明哪部分被丢、为什么。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigDiagnostics {
    entries: Vec<String>,
}

impl ConfigDiagnostics {
    /// 记一条。文案要自带位置（`[dictionaries]` / `[[custom_phrases]] 第 3 条`），用户照着能改回文件。
    pub fn push(&mut self, message: String) {
        self.entries.push(message);
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn messages(&self) -> &[String] {
        &self.entries
    }
}

impl fmt::Display for ConfigDiagnostics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.entries.join("；"))
    }
}
