//! 翻译组的翻译框：框里的原文、还在打的拼音与它的候选、底下一行状态。

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PanelTranslate {
    /// 框里已经打好的原文。
    pub source: String,

    /// 光标在原文里的位置（第几个字之后）；`composing` 画在这里。
    pub cursor: usize,

    /// 框里没字时显示的提示。
    pub placeholder: String,

    /// 还在打的拼音（组句中的原样），带下划线画在光标处；没在打时为空。
    pub composing: String,

    /// 拼音当前一页的候选。
    pub candidates: Vec<String>,

    /// 高亮候选在 `candidates` 里的下标。
    pub highlight: usize,

    /// 底下一行：目标语言与翻译进度（「译成英语 · 翻译中…」）。
    pub status: String,
}
