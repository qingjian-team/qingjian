//! [`Nearby`]：局域网里发现的设备。

use std::net::SocketAddr;

use crate::discovery::reach_rank;

/// 最多记几个地址：都连不上时每个要等一次连接超时。
const MAX_ADDRESSES: usize = 4;

/// 局域网里发现的一台设备，不一定配对过。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nearby {
    pub device_id: String,

    pub name: String,

    /// 型号或电脑名。
    pub model: String,

    /// 最可能连得上的地址，即 `addresses` 的第一个。
    pub address: SocketAddr,

    /// 它报过的地址，按 [`reach_rank`] 排、同档新的在前。每次通告只带一部分网卡的地址，只认最新一次会被虚拟网卡的地址顶掉。
    pub(crate) addresses: Vec<SocketAddr>,
}

impl Nearby {
    pub(crate) fn new(device_id: String, name: String, model: String, address: SocketAddr) -> Self {
        Self {
            device_id,
            name,
            model,
            address,
            addresses: vec![address],
        }
    }

    /// 记下又一次通告里的地址。
    pub(crate) fn remember(&mut self, address: SocketAddr) {
        self.addresses.retain(|known| *known != address);
        self.addresses.insert(0, address);
        self.addresses.sort_by_key(|known| reach_rank(known.ip()));
        self.addresses.truncate(MAX_ADDRESSES);
        self.address = self.addresses[0];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(address: &str) -> SocketAddr {
        address.parse().unwrap()
    }

    #[test]
    fn virtual_adapters_do_not_replace_the_lan_address() {
        let mut nearby = Nearby::new(
            "id".to_owned(),
            "电脑".to_owned(),
            String::new(),
            at("192.168.0.186:53319"),
        );
        nearby.remember(at("169.254.19.254:53319"));
        nearby.remember(at("198.18.0.1:53319"));
        nearby.remember(at("127.0.0.1:53319"));
        assert_eq!(nearby.address, at("192.168.0.186:53319"));
        assert_eq!(
            nearby.addresses,
            [
                at("192.168.0.186:53319"),
                at("198.18.0.1:53319"),
                at("169.254.19.254:53319"),
                at("127.0.0.1:53319"),
            ]
        );
    }

    #[test]
    fn newer_lan_address_goes_first_and_the_worst_is_dropped() {
        let mut nearby = Nearby::new(
            "id".to_owned(),
            "电脑".to_owned(),
            String::new(),
            at("169.254.19.254:53319"),
        );
        nearby.remember(at("192.168.0.186:53319"));
        nearby.remember(at("10.0.0.5:53319"));
        nearby.remember(at("8.8.8.8:53319"));
        nearby.remember(at("127.0.0.1:53319"));
        assert_eq!(
            nearby.addresses,
            [
                at("10.0.0.5:53319"),
                at("192.168.0.186:53319"),
                at("8.8.8.8:53319"),
                at("169.254.19.254:53319"),
            ]
        );
    }
}
