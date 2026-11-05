//! 清单现在是什么样：手上最新的一份（刚取到的或上次缓存的）和这次取的结果。

use std::time::SystemTime;

use super::{DictEntry, Status};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IndexState {
    /// 能显示的清单：这次取到的，没取到时是上次缓存的；都没有为空。
    pub dicts: Vec<DictEntry>,

    /// `dicts` 是什么时候取到的；没有清单为 `None`。
    pub fetched_at: Option<SystemTime>,

    pub status: Status,
}
