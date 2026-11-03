//! [`Stamp`]：一条内容的 id、时刻与删除状态，同步时只比较它。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stamp {
    pub id: String,

    /// 最后修改的 Unix 毫秒。
    pub at: i64,

    pub deleted: bool,

    /// 只是本机不要了（超出条数上限、清空）：别的设备不跟着删，本机也不会再把旧内容要回来。
    #[serde(default)]
    pub forgotten: bool,
}

impl Stamp {
    /// 比 `other` 新：时刻大的新；同一时刻删除优先，免得删了的又回来。
    pub fn newer_than(&self, other: &Stamp) -> bool {
        (self.at, self.deleted) > (other.at, other.deleted)
    }
}
