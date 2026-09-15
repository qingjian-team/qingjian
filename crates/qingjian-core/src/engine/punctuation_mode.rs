//! 组句中敲半角标点（`,` `.` `?` `-` 等）怎么办：进英文直输段、先把候选上屏、还是看缓冲区像不像英文再定。

use serde::{Deserialize, Serialize};

/// 组句中敲标点的处理方式，配置 `[general] punctuation_mode`。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PunctuationMode {
    /// 标点进缓冲区，整段成为英文直输段（`hello,` `no-way`），空格 / 回车原样上屏。中文模式下能直接打带标点的英文。
    #[default]
    Raw,

    /// 先把高亮候选上屏，标点再按没在组句时处理（中文模式转全角）：`nihao,` → 你好，。
    Commit,

    /// 缓冲区切不成完整拼音（`hello`）就进直输段，切得成（`nihao`）就先上屏再出标点；已在直输段里的继续追加。
    Auto,
}

impl PunctuationMode {
    /// 全部取值，设置界面按这个顺序列出。
    pub const ALL: [Self; 3] = [Self::Raw, Self::Commit, Self::Auto];

    /// 配置文件里的写法。
    pub fn key(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Commit => "commit",
            Self::Auto => "auto",
        }
    }

    /// 界面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Raw => "进入英文直输",
            Self::Commit => "先上屏候选再出标点",
            Self::Auto => "自动（像英文才直输）",
        }
    }
}
