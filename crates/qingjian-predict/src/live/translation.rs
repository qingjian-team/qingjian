//! 带修订标识的译文或可展示错误。

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Translation {
    pub id: u64,

    pub source: String,

    pub result: Result<String, String>,
}
