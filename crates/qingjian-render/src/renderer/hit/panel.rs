//! 符号面板上点中了哪一块。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelHit {
    /// 第 `n` 个分组胶囊。
    Group(usize),

    /// 当前组第 `n` 个分类标签。
    Section(usize),

    /// 视口里第 `n` 格（行优先，空格子不算点中）。
    Cell(usize),

    /// 翻译框的原文里点中的位置：光标该放到第 `n` 个字之后。
    Caret(usize),

    /// 翻译框里第 `n` 个候选。
    Candidate(usize),

    /// 右上角的关闭按钮。
    Close,
}
