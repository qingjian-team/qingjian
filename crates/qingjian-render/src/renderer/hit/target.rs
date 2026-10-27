//! 点中的是什么：第几个候选、第几个候选的第几条译词、翻页箭头，或右键菜单里第几项。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitTarget {
    /// 这一页第 `n` 个候选（从 0 起）。
    Candidate(usize),

    /// 这一页第 `candidate` 个候选右侧的第 `sense` 条译词（从 0 起，与「上屏第一 / 第二条译词」的键对应）。
    Translation { candidate: usize, sense: usize },

    /// 主题里写了 `action` 的翻页箭头：-1 上一页、1 下一页，和翻页键一样（矩阵展开时翻一屏）。
    Page(isize),

    /// 右键菜单里第 `n` 项（从 0 起）。
    MenuItem(usize),
}
