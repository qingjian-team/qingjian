//! 一个翻译方向的模型来源与文件清单。

use serde::Deserialize;

use crate::Asset;

#[derive(Debug, Clone, Deserialize)]
pub struct Package {
    pub direction: String,

    pub revision: String,

    pub source: String,

    pub files: Vec<Asset>,
}
