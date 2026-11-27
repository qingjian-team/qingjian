//! 清单现在是什么样：手上最新的一份（刚取到的或上次缓存的）和这次取的结果。

use std::time::SystemTime;

use super::Status;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexState<T> {
    /// 能显示的清单：这次取到的，没取到时是上次缓存的，都没有时是 [`super::Listing::fallback`]。
    pub items: Vec<T>,

    /// `items` 是什么时候取到的；没取到过为 `None`。
    pub fetched_at: Option<SystemTime>,

    pub status: Status,
}

impl<T> Default for IndexState<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            fetched_at: None,
            status: Status::default(),
        }
    }
}
