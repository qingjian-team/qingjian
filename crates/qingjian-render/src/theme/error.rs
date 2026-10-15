//! 主题文件读不进来。

#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    /// 读不到 `theme.json`。
    #[error("cannot read theme file: {0}")]
    Io(#[from] std::io::Error),

    /// 不是合法的 JSON，或结构对不上（缺必填项、类型错）。
    #[error("invalid theme file: {0}")]
    Json(#[from] serde_json::Error),

    /// `extends` 指向的主题不存在。
    #[error("unknown base theme {id:?}")]
    UnknownBase { id: String },

    /// `extends` 链太长（多半是互相继承）。
    #[error("theme inheritance too deep at {id:?}")]
    TooDeep { id: String },
}
