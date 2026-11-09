//! [`Progress`]：一轮同步里收内容的进度。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progress {
    /// 有内容要收，马上开始。
    Receiving,

    /// 当前这一条收到了多少字节。
    Bytes { received: u64, total: u64 },
}
