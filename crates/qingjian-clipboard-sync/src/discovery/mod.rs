//! 局域网发现：用 mDNS 公布自己并找同类。公布的内容不验真，只用来找地址，真假靠连接时的握手判断。
//! 安卓不用这份（见壳的 NsdDiscovery.kt）。访客网络等可能拦组播，所以还留了手动填地址。

mod found;

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use mdns_sd::{Receiver, ServiceDaemon, ServiceEvent, ServiceInfo};

use crate::error::SyncError;

pub use found::Found;

const SERVICE_TYPE: &str = "_qjclip._tcp.local.";

/// DNS 单个标签最长 63，公钥十六进制有 64 位，实例名只取前 32 位，完整 id 放 TXT。
const INSTANCE_LENGTH: usize = 32;

/// TXT 单项上限 255 字节，设备名超了截断。
const MAX_NAME_BYTES: usize = 180;

const PROTOCOL_VERSION: &str = "1";

pub struct Discovery {
    daemon: ServiceDaemon,

    events: Receiver<ServiceEvent>,

    own_id: String,

    /// mDNS 全名到设备 id：下线事件只带全名。
    known: HashMap<String, String>,
}

impl Discovery {
    pub fn start(device_id: &str, name: &str, model: &str, port: u16) -> Result<Self, SyncError> {
        let daemon = ServiceDaemon::new().map_err(mdns_error)?;
        let instance = &device_id[..INSTANCE_LENGTH.min(device_id.len())];
        let host = format!("qj-{instance}.local.");
        let name = truncate(name, MAX_NAME_BYTES);
        let properties = [
            ("id", device_id),
            ("name", name),
            ("model", truncate(model, MAX_NAME_BYTES)),
            ("v", PROTOCOL_VERSION),
        ];
        let info = ServiceInfo::new(SERVICE_TYPE, instance, &host, "", port, &properties[..])
            .map_err(mdns_error)?
            .enable_addr_auto();
        daemon.register(info).map_err(mdns_error)?;
        let events = daemon.browse(SERVICE_TYPE).map_err(mdns_error)?;
        Ok(Self {
            daemon,
            events,
            own_id: device_id.to_owned(),
            known: HashMap::new(),
        })
    }

    /// 等下一个变化，超时返回 `None`。
    pub fn next(&mut self, timeout: Duration) -> Option<Found> {
        match self.events.recv_timeout(timeout).ok()? {
            ServiceEvent::ServiceResolved(service) => {
                if service.get_property_val_str("v") != Some(PROTOCOL_VERSION) {
                    return None;
                }
                let device_id = service.get_property_val_str("id")?.to_owned();
                if device_id == self.own_id {
                    return None;
                }
                let name = service
                    .get_property_val_str("name")
                    .unwrap_or_default()
                    .to_owned();
                let model = service
                    .get_property_val_str("model")
                    .unwrap_or_default()
                    .to_owned();
                let mut addresses: Vec<_> = service
                    .get_addresses_v4()
                    .into_iter()
                    .map(IpAddr::V4)
                    .collect();
                addresses.sort_by_key(|address| !is_private(address));
                let address = SocketAddr::new(*addresses.first()?, service.get_port());
                self.known
                    .insert(service.get_fullname().to_owned(), device_id.clone());
                Some(Found::Seen {
                    device_id,
                    name,
                    model,
                    address,
                })
            }
            ServiceEvent::ServiceRemoved(_, fullname) => {
                let device_id = self.known.remove(&fullname)?;
                Some(Found::Gone { device_id })
            }
            _ => None,
        }
    }
}

impl Drop for Discovery {
    fn drop(&mut self) {
        if let Err(error) = self.daemon.shutdown() {
            tracing::debug!(%error, "mdns shutdown failed");
        }
    }
}

fn mdns_error(error: mdns_sd::Error) -> SyncError {
    SyncError::Storage(format!("mdns: {error}"))
}

fn is_private(address: &IpAddr) -> bool {
    matches!(address, IpAddr::V4(v4) if v4.is_private())
}

/// 按字节截断，不切开一个字。
fn truncate(text: &str, limit: usize) -> &str {
    let mut end = limit.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_keeps_whole_characters() {
        assert_eq!(truncate("手机", 4), "手");
        assert_eq!(truncate("手机", 100), "手机");
    }
}
