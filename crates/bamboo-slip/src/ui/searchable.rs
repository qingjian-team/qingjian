//! 能被搜到的一项：设置行、分组标题、勾选项建树时登记标题与说明，使用方据此做设置搜索。

use super::Key;

#[derive(Debug, Clone, PartialEq)]
pub struct Searchable {
    /// 那一项的元素 key：搜到后交给 [`crate::App::spotlight`] 滚进视野、醒目标一下。
    pub key: Key,

    pub title: String,

    /// 补充说明（设置行的 ⓘ 全文）。
    pub detail: Option<String>,

    /// 行里控件上的字（分段的各项、下拉框的当前值与全部选项）与使用方补的同义词：记不清标题时按它也能搜到。
    pub keywords: Vec<String>,
}
