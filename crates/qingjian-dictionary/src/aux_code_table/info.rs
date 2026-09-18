//! 码表页要显示的元数据，读不动也不报错。

use std::path::Path;

use super::table::AuxCodeTable;

/// 码表页要显示的元数据：名称、条数、许可、是不是坏文件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuxCodeTableInfo {
    /// 给人看的名称（`.qj` 元数据里的 name，取不到时是文件名主干）。
    pub name: String,

    /// 词条数（词 → 码 的条目数）。
    pub entries: usize,

    /// 许可证标识（SPDX）。
    pub license: String,

    /// 打开失败（不是码表、文件损坏）时为真，界面标红。
    pub broken: bool,
}

/// 读码表的元数据。打不开或不是码表时不报错：返回 `broken = true`，名称退回文件名主干，
/// 设置页照样把它列出来（用户能看见并删掉这个坏文件）。
pub fn aux_code_table_info(path: &Path) -> AuxCodeTableInfo {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_owned();
    match AuxCodeTable::open(path) {
        Ok(table) => {
            let metadata = table.metadata();
            AuxCodeTableInfo {
                name: metadata
                    .map(|m| m.name.clone())
                    .filter(|name| !name.is_empty())
                    .unwrap_or(stem),
                entries: table.len(),
                license: metadata.map_or_else(String::new, |m| m.license.clone()),
                broken: false,
            }
        }
        Err(error) => {
            tracing::warn!(file = %path.display(), %error, "码表打不开");
            AuxCodeTableInfo {
                name: stem,
                entries: 0,
                license: String::new(),
                broken: true,
            }
        }
    }
}
