//! 词库页的内容。

use serde::Serialize;

use super::DictionaryEntry;

#[derive(Debug, Serialize)]
pub struct DictionaryPage {
    /// 随包的领域词库，按文件名排。
    pub domains: Vec<DictionaryEntry>,

    /// 导入的词库，按文件名排。
    pub imported: Vec<DictionaryEntry>,

    /// 这次导入 / 删除的结果，一句话给用户看；只列时为空。
    pub message: String,
}
