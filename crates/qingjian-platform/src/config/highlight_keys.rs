//! `[shortcut] highlight_keys`：候选高亮由 `↑` / `↓` 还是 `←` / `→` 移动。

use serde::{Deserialize, Serialize};

/// 候选高亮由哪对方向键移动（`[shortcut] highlight_keys`，只有 Windows 用）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HighlightKeys {
    /// `↑` / `↓` 移动高亮，`←` / `→` 移动拼音光标（缺省，与以前一致）。
    #[default]
    UpDown,

    /// `←` / `→` 移动高亮，`↑` / `↓` 不做事。拼音行在候选栏左边，两键把「拼音行 + 候选栏」
    /// 当成一条从左到右的序列走：`→` 先移拼音光标、到末位溢出成选词，`←` 先退候选、到首位回拼音光标。
    LeftRight,
}

impl HighlightKeys {
    /// 全部取值，设置界面按这个顺序列出。
    pub const ALL: [Self; 2] = [Self::UpDown, Self::LeftRight];

    /// 配置文件里的写法。
    pub fn key(self) -> &'static str {
        match self {
            Self::UpDown => "updown",
            Self::LeftRight => "leftright",
        }
    }

    /// 界面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::UpDown => "↑ / ↓",
            Self::LeftRight => "← / →",
        }
    }
}
