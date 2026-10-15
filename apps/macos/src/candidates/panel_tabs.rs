//! 符号面板顶上的两行标签：分组（搜索 / 符号 / 表情）与当前组的分类。

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PanelTabs {
    /// 分组名，按顺序。
    pub groups: Vec<String>,

    /// 当前分组在 `groups` 里的下标。
    pub group: usize,

    /// 当前分组的分类名。
    pub sections: Vec<String>,

    /// 当前分类在 `sections` 里的下标。
    pub section: usize,
}
