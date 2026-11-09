//! 一条历史。格式的字节不在这里，要用时按 id 取，列表里不用把图片都读出来。

use crate::kind::ClipKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipItem {
    /// 全局唯一（设备 id + 序号），多端同步时靠它认同一条。
    pub id: String,

    pub kind: ClipKind,

    pub text: String,

    /// 来源程序的路径，不知道时为空。
    pub source: String,

    /// 最后一次复制或修改的时刻（Unix 毫秒）；列表按它排序。
    pub updated_at: i64,
}
