//! [`Found`]：局域网发现的一次变化。

use std::net::SocketAddr;

/// 发现的变化。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    Seen {
        device_id: String,

        name: String,

        /// 型号或电脑名，名字看不出是哪台时靠它。
        model: String,

        address: SocketAddr,
    },
    Gone {
        device_id: String,
    },
}
