//! 「翻译选中文字」的进行态。

/// 一次「翻译选中文字」的进行态。
#[derive(Default)]
pub(crate) struct Translation {
    /// 翻译提供方返回的译文。
    pub(crate) result: Option<String>,

    /// 翻译失败的提示。
    pub(crate) status: Option<String>,
}
