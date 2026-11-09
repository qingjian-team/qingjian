//! 按音节出字的请求：同一段输入只算一次，结果按 `key` 缓存。

/// 一次出字请求。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ComposeRequest {
    /// 缓存键：前文与音节串拼起来，见 [`super::compose_key`]。
    pub key: String,

    pub context: String,

    /// 敲的每个音节与它敲全了没有。
    pub syllables: Vec<(String, bool)>,
}
