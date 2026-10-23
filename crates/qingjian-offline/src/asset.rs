//! 一个固定版本、固定摘要的模型文件。

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    pub name: String,

    pub url: String,

    pub bytes: u64,

    pub sha256: String,
}
