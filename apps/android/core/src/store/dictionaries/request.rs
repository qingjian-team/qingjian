//! 词库页交过来的请求：列出之前先导入或删除一本。

use serde::Deserialize;

/// `{"import":"/data/.../cache/import/law.dict.yaml"}`、`{"remove":"law"}`，都不带就只列。
#[derive(Debug, Default, Deserialize)]
pub struct DictionaryRequest {
    /// 要导入的文件（壳从系统文件选择器拷到缓存目录，保留原文件名）。
    #[serde(default)]
    pub import: Option<String>,

    /// 要删除的导入词库的 id。
    #[serde(default)]
    pub remove: Option<String>,
}
