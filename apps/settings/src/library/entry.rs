//! 清单里的一本词库或码表。

#[derive(Debug, Clone)]
pub struct Entry {
    /// 文件名（不含扩展名），配置里按它开关。
    pub stem: String,

    pub name: String,

    pub entries: usize,

    pub license: String,

    /// 打不开的坏文件：照样列出来好让人移除。
    pub broken: bool,
}
