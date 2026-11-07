//! 输入框的编号：编辑状态按它存在 `Settings::fields` 里。

/// 输入框：云服务页的三个，中英文页的保持半角符号，短语编辑卡片里的三个，生词本页的搜索框。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldId {
    BaseUrl,
    Model,
    ApiKey,

    /// 中英文页「保持半角的符号」（`[general] half_width_symbols`）。
    HalfWidthSymbols,
    PhraseCode,
    PhraseWeight,
    PhraseText,
    /// 生词本页的搜索框，不存进配置。
    VocabularySearch,
}

impl FieldId {
    /// 存进配置的那几个（短语的几个随卡片一起存）。
    pub const ALL: [Self; 4] = [
        Self::BaseUrl,
        Self::Model,
        Self::ApiKey,
        Self::HalfWidthSymbols,
    ];

    /// 配置文件里的键（[`Self::HalfWidthSymbols`] 在 `[general]`，其余在 `[predict]`）。
    pub fn key(self) -> &'static str {
        match self {
            Self::BaseUrl => "base_url",
            Self::Model => "model",
            Self::ApiKey => "api_key",
            Self::HalfWidthSymbols => "half_width_symbols",
            Self::PhraseCode => "code",
            Self::PhraseWeight => "weight",
            Self::PhraseText => "text",
            Self::VocabularySearch => "vocabulary-search",
        }
    }
}
