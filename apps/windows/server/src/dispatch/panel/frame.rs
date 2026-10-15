//! 面板一次要画的内容，Router 产出、UI 线程画。

use qingjian_platform::Appearance;

/// 面板一帧：两行标签、视口里的格子与信息行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelFrame {
    /// 分组名（搜索 / 符号 / 表情），按顺序。
    pub groups: Vec<String>,

    /// 当前分组在 `groups` 里的下标。
    pub group: usize,

    /// 当前分组的分类名。
    pub sections: Vec<String>,

    /// 当前分类在 `sections` 里的下标。
    pub section: usize,

    /// 视口里的格子，按行优先排开，空位是 `None`。
    pub cells: Vec<Option<PanelCell>>,

    /// 每行几格。
    pub columns: usize,

    /// 高亮在 `cells` 里的下标。
    pub highlight: usize,

    /// 「第几行 / 共几行」，只有一屏时没有。
    pub footer: Option<String>,

    pub appearance: Appearance,
}

/// 一格：条目本身与它的中文名字（信息行里显示）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelCell {
    pub text: String,

    pub name: Option<String>,
}
