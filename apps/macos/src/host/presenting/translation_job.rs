use objc2_foundation::NSRange;

/// 一次「翻译选中文字」：从按下快捷键到用户接受或放弃。
#[derive(Debug, Clone)]
pub struct TranslationJob {
    /// 选区在应用里的范围，接受时用译文替换它。
    pub range: NSRange,

    /// 云端回来的译文；`None` 表示还在等。
    pub result: Option<String>,
}
