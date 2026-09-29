//! macOS 上方竖排候选倒序显示时，方向键按候选顺序还是屏幕方向移动。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerticalAboveArrowKeys {
    /// 保留原来的上一项 / 下一项按键习惯。
    #[default]
    Candidate,

    /// 高亮按屏幕上的上 / 下方向移动。
    Visual,
}

impl VerticalAboveArrowKeys {
    pub const ALL: [Self; 2] = [Self::Candidate, Self::Visual];

    pub fn key(self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::Visual => "visual",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Candidate => "按候选顺序",
            Self::Visual => "按屏幕方向",
        }
    }
}
