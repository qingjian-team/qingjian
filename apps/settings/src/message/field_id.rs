//! 输入框的编号：编辑状态按它存在 `Settings::fields` 里。

/// 输入框：云服务页的三个，短语编辑卡片里的两个，生词本页的搜索框。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldId {
    BaseUrl,
    Model,
    ApiKey,
    PhraseCode,
    PhraseText,
    /// 生词本页的搜索框，不存进配置。
    VocabularySearch,
}

impl FieldId {
    /// 存进配置的那几个（短语的两个随卡片一起存）。
    pub const ALL: [Self; 3] = [Self::BaseUrl, Self::Model, Self::ApiKey];

    /// 配置文件 `[predict]` 里的键。
    pub fn key(self) -> &'static str {
        match self {
            Self::BaseUrl => "base_url",
            Self::Model => "model",
            Self::ApiKey => "api_key",
            Self::PhraseCode => "code",
            Self::PhraseText => "text",
            Self::VocabularySearch => "vocabulary-search",
        }
    }
}
