//! 符号面板一帧的内容：顶上分组与分类两行标签，中间视口里的格子，底下信息行。由壳按 Core 的面板状态填好交来，
//! [`crate::Renderer::render_panel`] 按主题的 `panel` 一节画。

mod cell;
mod translate;

pub use cell::PanelCell;
pub use translate::PanelTranslate;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
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

    /// 每行几格；少于主题一行的格数时格子拉宽（颜文字），总宽不变。
    pub columns: usize,

    /// 视口几行；面板高度按它定，内容少的分类下面留空，换分类窗口不跳。
    pub rows: usize,

    /// 高亮在 `cells` 里的下标。
    pub highlight: usize,

    /// 「第几行 / 共几行」，只有一屏时没有。
    pub footer: Option<String>,

    /// 顶上一行右边的按键提示。
    pub hint: String,

    /// 翻译组：有它时格子区换成翻译框，`cells` 与 `footer` 不画。
    pub translate: Option<PanelTranslate>,
}
