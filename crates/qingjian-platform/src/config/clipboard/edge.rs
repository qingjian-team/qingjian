//! [`ClipboardEdge`]：面板从屏幕哪条边弹出。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClipboardEdge {
    Top,
    #[default]
    Bottom,
    Left,
    Right,
}

impl ClipboardEdge {
    /// 全部取值，设置界面按这个顺序列出。
    pub const ALL: [Self; 4] = [Self::Top, Self::Bottom, Self::Left, Self::Right];

    /// 配置文件里的写法。
    pub fn key(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}
