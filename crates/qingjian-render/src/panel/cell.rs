//! 面板的一格。

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelCell {
    /// 条目本身（符号、emoji、颜文字）。
    pub text: String,

    /// 中文名字，高亮时在信息行里显示。
    pub name: Option<String>,
}
