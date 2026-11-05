//! 清单里的一本词库或码表。

#[derive(Debug, Clone)]
pub struct Entry {
    /// 文件名（不含扩展名），配置里按它开关。
    pub stem: String,

    pub name: String,

    pub entries: usize,

    pub license: String,

    /// 元数据里写的数据版本（从词库仓库下的是 `dict.toml` 的 `version`），没写是空的；「词库」页据此显示「更新」。
    pub version: String,

    /// 打不开的坏文件：照样列出来好让人移除。
    pub broken: bool,
}
