use serde::{Deserialize, Serialize};

/// 数据文件的来历：名称、许可证、署名、来源。写在每个 `.qj` 的第一节，偏好设置里的词库列表直接显示它。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
    /// 给人看的名称（「雾凇拼音」「青简基础词库」）。
    pub name: String,

    /// 许可证标识（SPDX 表达式，如 `GPL-3.0-only`、`CC-BY-SA-4.0`）。
    #[serde(default)]
    pub license: String,

    /// 署名 / 版权行。
    #[serde(default)]
    pub attribution: String,

    /// 来源 URL。
    #[serde(default)]
    pub source: String,

    /// 数据版本（上游版本号或日期）。
    #[serde(default)]
    pub version: String,

    /// 条数（词库是词条数，语言模型是 bigram 对数），只用于显示。
    #[serde(default)]
    pub entries: u64,

    /// 谁生成的（`qingjian-dict-convert 0.1.0`）。
    #[serde(default)]
    pub generator: String,
}

impl Metadata {
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string(self)
    }

    pub fn from_toml(text: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(text)
    }
}
