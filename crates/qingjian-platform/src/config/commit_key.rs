//! 组句中上屏高亮候选用空格还是回车（`[general] commit_key`）。

use serde::{Deserialize, Serialize};

/// 拼音 / 双拼组句时，哪个键上屏高亮候选（另一个上屏原始字母）。
/// 注音方案下回车本来就上屏候选、空格是一声，不受此项影响。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommitKey {
    /// 空格上屏候选，回车上屏拼音（缺省，与以前一致）。
    #[default]
    Space,

    /// 回车上屏候选，空格上屏拼音。
    Enter,
}

impl CommitKey {
    /// 全部取值，设置界面按这个顺序列出。
    pub const ALL: [Self; 2] = [Self::Space, Self::Enter];

    /// 配置文件里的写法。
    pub fn key(self) -> &'static str {
        match self {
            Self::Space => "space",
            Self::Enter => "enter",
        }
    }

    /// 界面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Space => "空格",
            Self::Enter => "回车",
        }
    }

    /// 回车是否上屏高亮候选（拼音 / 双拼）。
    pub fn enter_commits(self) -> bool {
        matches!(self, Self::Enter)
    }
}

#[cfg(test)]
mod tests {
    use super::CommitKey;

    #[test]
    fn keys_round_trip_through_lowercase() {
        assert_eq!(CommitKey::default(), CommitKey::Space);
        assert_eq!(CommitKey::Space.key(), "space");
        assert_eq!(CommitKey::Enter.key(), "enter");
        assert!(!CommitKey::Space.enter_commits());
        assert!(CommitKey::Enter.enter_commits());
    }
}
