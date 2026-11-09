//! 面板目录的一个分类。

use serde::Serialize;

/// 一个分类：所在分组（「符号」「表情」）、名字与条目。
#[derive(Debug, Serialize)]
pub struct CatalogSection {
    pub group: &'static str,

    pub name: &'static str,

    pub items: Vec<&'static str>,
}
