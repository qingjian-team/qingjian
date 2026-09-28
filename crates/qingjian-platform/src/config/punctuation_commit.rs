//! 组句中敲半角标点的处理方式。

use serde::{Deserialize, Serialize};

/// 组句中敲半角标点（翻页键除外）怎么处理。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PunctuationCommit {
    /// 上屏当前高亮候选并补出对应的全角标点（`nihao,` → 你好，），主流输入法习惯。
    /// 想在中文模式里直接打 `hello,` 这类带标点的英文请用 `raw`，或临时按 Caps 进英文模式。
    Candidate,

    /// 标点进缓冲区，整段成为英文直输段、原样上屏：中文模式不打切换键也能打带标点的英文。
    Raw,
}

impl Default for PunctuationCommit {
    fn default() -> Self {
        Self::Candidate
    }
}

impl PunctuationCommit {
    /// 从配置字符串解析；不认识的写法按缺省并警告。
    pub fn parse(value: &str) -> Self {
        match value {
            "candidate" => Self::Candidate,
            "raw" => Self::Raw,
            other => {
                tracing::warn!(value = %other, "punctuation_commit 取值不认识，按 candidate 处理");
                Self::Candidate
            }
        }
    }

    pub fn is_candidate(self) -> bool {
        self == Self::Candidate
    }
}
