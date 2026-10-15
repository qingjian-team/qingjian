//! 搜索索引里的一条。

use bamboo_slip::Key;

use crate::page_id::PageId;

/// 一条可搜的内容：在哪页、哪个元素（页名本身没有）、标题与说明。
#[derive(Debug, Clone)]
pub struct Entry {
    pub page: PageId,

    pub key: Option<Key>,

    pub title: String,

    pub detail: Option<String>,

    /// 行里控件上的字与同义词。
    pub keywords: Vec<String>,
}
