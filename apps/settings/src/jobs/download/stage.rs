//! 下载走到哪了。

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stage {
    /// 排着队，前面的下完才轮到。
    Queued,

    Downloading {
        done: u64,
        total: u64,
    },

    /// 下完、核对过，在解压换进下载目录。
    Installing,

    Done,

    Failed(String),
}
