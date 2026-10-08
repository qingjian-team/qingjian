//! 英语译词的学习难度，按 CEFR 分成三档。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TranslationDifficulty {
    #[default]
    All,

    Beginner,

    Intermediate,

    Advanced,
}

impl TranslationDifficulty {
    pub const ALL: [Self; 4] = [
        Self::All,
        Self::Beginner,
        Self::Intermediate,
        Self::Advanced,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Beginner => "beginner",
            Self::Intermediate => "intermediate",
            Self::Advanced => "advanced",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "全部难度",
            Self::Beginner => "基础（A1–A2）",
            Self::Intermediate => "进阶（B1–B2）",
            Self::Advanced => "高级（C1–C2）",
        }
    }

    pub fn matches(self, level: &str) -> bool {
        match self {
            Self::All => true,
            Self::Beginner => matches!(level, "A1" | "A2"),
            Self::Intermediate => matches!(level, "B1" | "B2"),
            Self::Advanced => matches!(level, "C1" | "C2"),
        }
    }
}
