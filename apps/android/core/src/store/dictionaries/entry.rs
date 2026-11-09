//! 词库页上的一行。

use serde::Serialize;

/// `id` 是文件名去掉扩展名，开关与删除都按它；`name` 是词库自带的名字（领域词库去掉统一的前缀）。
#[derive(Debug, Serialize)]
pub struct DictionaryEntry {
    pub id: String,

    pub name: String,

    pub entries: usize,

    /// 许可证，没写为空。
    pub license: String,
}
