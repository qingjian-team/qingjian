use serde::{Deserialize, Serialize};

/// 候选窗口排布。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LayoutMode {
    /// 竖排：一行一个候选，译文在同一行右侧。
    #[default]
    Vertical,

    /// 横排：候选排成一行，只给高亮那个在下面显示译文。
    Horizontal,
}

impl LayoutMode {
    /// 全部取值，设置界面按这个顺序列出。
    pub const ALL: [Self; 2] = [Self::Vertical, Self::Horizontal];

    /// 配置文件里的写法。
    pub fn key(self) -> &'static str {
        match self {
            Self::Vertical => "vertical",
            Self::Horizontal => "horizontal",
        }
    }

    /// 界面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Vertical => "竖排",
            Self::Horizontal => "横排",
        }
    }
}
