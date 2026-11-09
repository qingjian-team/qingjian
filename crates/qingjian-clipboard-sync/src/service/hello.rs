//! [`PairHello`]：配对成功后两边互报的名字和监听端口。发起连接用的是临时端口，不互报就连不回去。

use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use crate::error::SyncError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct PairHello {
    pub name: String,

    pub port: u16,
}

impl PairHello {
    pub fn encode(&self) -> Result<Vec<u8>, SyncError> {
        Ok(serde_json::to_vec(self)?)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, SyncError> {
        Ok(serde_json::from_slice(bytes)?)
    }

    /// 对方能被连上的地址：连接来源的 IP 加对方报的端口。
    pub fn address_from(&self, source: SocketAddr) -> String {
        SocketAddr::new(source.ip(), self.port).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::PairHello;

    #[test]
    fn address_uses_the_reported_port_not_the_source_port() {
        let hello = PairHello {
            name: "手机".to_owned(),
            port: 53319,
        };
        let back = PairHello::decode(&hello.encode().unwrap()).unwrap();
        assert_eq!(back, hello);
        let source = "192.168.0.5:61234".parse().unwrap();
        assert_eq!(back.address_from(source), "192.168.0.5:53319");
    }
}
