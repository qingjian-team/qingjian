//! 输入状态：主题用它显示「中 / 英」「简 / 繁」这类指示。显示什么字由主题定（`when: "mode.english"` 分两个文字节点），
//! 渲染器只给事实；方案名由壳给好显示名，主题用 `"bind": "mode.scheme"` 显示。

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Mode {
    /// 英文模式（`false` 为中文）。
    pub english: bool,

    /// 繁体输出。
    pub traditional: bool,

    /// 当前模式下标点是全角。
    pub full_width: bool,

    /// 输入方案的显示名，如「全拼」「小鹤双拼」「五笔（86） + 全拼」。
    pub scheme: String,
}
