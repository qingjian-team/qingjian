//! [`Nearby`]：局域网里发现的设备。

use std::net::SocketAddr;

/// 局域网里发现的一台设备，不一定配对过。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nearby {
    pub device_id: String,

    pub name: String,

    /// 型号或电脑名。
    pub model: String,

    pub address: SocketAddr,
}
