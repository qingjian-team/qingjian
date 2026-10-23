//! 「翻译选中文字」的进行态。

/// 一次「翻译选中文字」的进行态。
pub(crate) struct Translation {
    /// 云端译文；`None` 表示还在等（候选窗显示「翻译中…」）。
    pub(crate) result: Option<String>,

    /// 备用来源只允许复制译文，不能把它当作 TSF 文档范围。
    pub(crate) copy_only: bool,

    pub(super) reading: Option<super::reading::ReadJob>,

    /// 阶段提示或失败原因，禁止在这里拼入原文。
    pub(crate) status: Option<String>,

    pub(crate) copied: bool,
}

impl Translation {
    pub(super) fn new(copy_only: bool) -> Self {
        Self {
            result: None,
            copy_only,
            reading: None,
            status: None,
            copied: false,
        }
    }
}
