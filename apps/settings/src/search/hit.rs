//! 一条搜索结果。

use bamboo_slip::Key;

use crate::page_id::PageId;

#[derive(Debug, Clone)]
pub struct Hit {
    pub page: PageId,

    /// 跳过去要醒目标出的元素；页名本身没有。
    pub key: Option<Key>,

    pub title: String,

    /// 说明里命中时，命中处前后几个字。
    pub snippet: Option<String>,

    /// 收在哪张展开卡片里，跳过去前先展开。
    pub region: Option<Key>,
}
