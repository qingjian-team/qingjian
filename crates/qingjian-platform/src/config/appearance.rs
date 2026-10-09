use serde::{Deserialize, Serialize};

/// 候选窗口外观：浅色 / 深色，或跟随系统。与选哪个主题（`[general] theme`）无关，每个主题都有两套颜色。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    /// 跟随系统。
    #[default]
    System,

    /// 始终浅色。
    Light,

    /// 始终深色。
    Dark,
}

impl Appearance {
    /// 全部取值，设置界面按这个顺序列出。
    pub const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];

    /// 配置文件里的写法。
    pub fn key(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// 界面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::System => "跟随系统",
            Self::Light => "浅色",
            Self::Dark => "深色",
        }
    }

    /// 按配置文件里的写法找，不认识为 `None`。
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|appearance| appearance.key() == key)
    }
}
