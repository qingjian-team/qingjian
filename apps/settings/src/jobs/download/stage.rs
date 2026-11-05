//! 下载走到哪了。

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stage {
    Downloading {
        done: u64,
        total: u64,
    },

    /// 下完、核对过，在解压换进用户目录。
    Installing,

    Done,

    Failed(String),
}
