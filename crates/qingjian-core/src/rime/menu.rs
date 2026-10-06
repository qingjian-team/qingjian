//! Rime 拥有分页与高亮；壳只画当前页，不对候选重新排序。
#[derive(Debug, Clone, Default)]
pub struct RimeMenu {
    pub page: usize,

    pub page_size: usize,

    pub last_page: bool,

    pub highlighted: usize,

    /// 预编辑文本中的字符位置，已从 librime 的 UTF-8 字节位置转换。
    pub cursor: usize,
}
