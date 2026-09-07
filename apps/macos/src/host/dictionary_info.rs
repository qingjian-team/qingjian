use std::path::PathBuf;

/// 偏好设置「词库」页里的一行：一本随包领域词库，或用户目录 `dicts/` 下的一本导入词库。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictionaryInfo {
    /// 文件名主干，配置 `[dictionaries]` 的 `domains` / `disabled` 里用它。
    pub stem: String,

    /// 随包的领域词库：开关记在 `domains`，不能移除。
    pub builtin: bool,

    /// 文件路径。
    pub path: PathBuf,

    /// 显示名（`.qj` 的 META 名称，否则文件名主干）。
    pub name: String,

    /// 词条数；文件坏了是 0。
    pub entries: usize,

    /// 许可证（META 里的），没有为空。
    pub license: String,

    /// 是否启用。
    pub enabled: bool,

    /// 文件读不出来（坏文件）。
    pub broken: bool,
}
