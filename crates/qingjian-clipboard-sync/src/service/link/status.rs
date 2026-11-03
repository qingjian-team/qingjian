//! [`PeerStatus`]：已配对设备现在连不连得上。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerStatus {
    /// 最近真的连上过。
    Connected,

    /// 还没有结果，正在试。
    Connecting,

    /// 最近一次连接失败了。
    Unreachable,
}
