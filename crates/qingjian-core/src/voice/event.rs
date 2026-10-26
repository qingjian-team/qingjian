//! 识别过程中交回给壳的一件事。

/// [`super::Transcriber::poll`] 的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscriptEvent {
    /// 到目前为止听出来的文字（整段，不是增量），说话途中会一再更新。
    Partial(String),

    /// 这一段的最终结果；之后不再有事件。
    Final(String),

    /// 出错（没网、模型加载失败、没听到声音），给用户看的一句话；之后不再有事件。
    Error(String),
}
