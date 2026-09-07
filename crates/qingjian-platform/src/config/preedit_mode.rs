use serde::{Deserialize, Serialize};

/// 组句中的拼音（preedit）显示在哪里。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PreeditMode {
    /// 行内 marked text 与候选窗口顶部都显示。
    #[default]
    Both,

    /// 只在行内（应用里的 marked text），候选窗口不带拼音行。
    Inline,

    /// 只在候选窗口顶部，应用里不放 marked text（终端、部分 Electron 应用画不好行内拼音时用）。
    Window,
}

impl PreeditMode {
    /// 全部取值，设置界面按这个顺序列出。
    pub const ALL: [Self; 3] = [Self::Both, Self::Inline, Self::Window];

    /// 配置文件里的写法。
    pub fn key(self) -> &'static str {
        match self {
            Self::Both => "both",
            Self::Inline => "inline",
            Self::Window => "window",
        }
    }

    /// 界面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Both => "行内 + 候选窗口",
            Self::Inline => "只在行内",
            Self::Window => "只在候选窗口",
        }
    }

    /// 要不要往应用里放 marked text。
    pub fn inline(self) -> bool {
        !matches!(self, Self::Window)
    }

    /// 候选窗口顶部要不要画拼音行。
    pub fn in_window(self) -> bool {
        !matches!(self, Self::Inline)
    }
}
