//! [`Peer`]：一台已配对的设备。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Peer {
    /// 公钥的十六进制，也是设备 id。
    pub device_id: String,

    pub name: String,

    /// 上次连上的地址，发现不了时先试它。
    pub last_address: Option<String>,

    /// 上次真正连上的时刻（毫秒）。
    #[serde(default)]
    pub last_seen: Option<i64>,
}
