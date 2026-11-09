//! 按需扫描：直接连同网段每个地址的同步端口，问它是谁。路由器拦组播、mDNS 发现不了时靠它列出设备，对方换了 IP 也靠它找回来。
//! 回答只有设备 id、名字和型号（mDNS 本来就公布这些），不碰剪贴板内容。只扫首选端口，被占用改用别的端口的设备扫不到。

use std::collections::HashSet;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::state::Shared;
use super::{Command, PREFERRED_PORT, apply_found, lock};
use crate::channel::MODE_INFO;
use crate::discovery::Found;
use crate::error::SyncError;

const PROBE_TIMEOUT: Duration = Duration::from_millis(400);

const WORKERS: usize = 64;

const PROTOCOL_VERSION: u32 = 1;

/// 回答的长度上限。
const MAX_INFO: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct DeviceInfo {
    pub id: String,

    pub name: String,

    pub model: String,

    pub version: u32,
}

/// 回答别的设备的询问（开头那个字节已读过）。
pub(super) fn answer<S: Write>(stream: &mut S, shared: &Shared) -> Result<(), SyncError> {
    let body = serde_json::to_vec(&DeviceInfo {
        id: shared.identity.device_id(),
        name: shared.name.clone(),
        model: shared.model.clone(),
        version: PROTOCOL_VERSION,
    })?;
    let length =
        u16::try_from(body.len()).map_err(|_| SyncError::Protocol("info too large".to_owned()))?;
    stream.write_all(&length.to_be_bytes())?;
    stream.write_all(&body)?;
    stream.flush()?;
    Ok(())
}

/// 问一个地址；不是同类或连不上都是错误。
pub(super) fn probe(address: SocketAddr) -> Result<DeviceInfo, SyncError> {
    let mut stream = TcpStream::connect_timeout(&address, PROBE_TIMEOUT)?;
    stream.set_read_timeout(Some(PROBE_TIMEOUT))?;
    stream.set_write_timeout(Some(PROBE_TIMEOUT))?;
    stream.write_all(&[MODE_INFO])?;
    let mut length = [0_u8; 2];
    stream.read_exact(&mut length)?;
    let length = usize::from(u16::from_be_bytes(length));
    if length > MAX_INFO {
        return Err(SyncError::Protocol("info too large".to_owned()));
    }
    let mut body = vec![0_u8; length];
    stream.read_exact(&mut body)?;
    let info: DeviceInfo = serde_json::from_slice(&body)?;
    if info.version == PROTOCOL_VERSION {
        Ok(info)
    } else {
        Err(SyncError::Protocol("other version".to_owned()))
    }
}

/// 同一个 /24 里除自己以外的地址。掩码更短的网段也只扫这一段：整个 /16 扫不完。
fn neighbours(own: Ipv4Addr) -> Vec<Ipv4Addr> {
    let [a, b, c, d] = own.octets();
    (1..=254)
        .filter(|host| *host != d)
        .map(|host| Ipv4Addr::new(a, b, c, host))
        .collect()
}

fn local_targets() -> Vec<SocketAddr> {
    let mut targets = Vec::new();
    for interface in if_addrs::get_if_addrs().unwrap_or_default() {
        if let IpAddr::V4(own) = interface.ip()
            && own.is_private()
        {
            targets.extend(
                neighbours(own)
                    .into_iter()
                    .map(|host| SocketAddr::new(IpAddr::V4(host), PREFERRED_PORT)),
            );
        }
    }
    targets.sort();
    targets.dedup();
    targets
}

/// 在后台扫一遍；已经在扫就不重复。
pub(super) fn start(shared: Arc<Shared>, commands: Sender<Command>) {
    if shared.scanning.swap(true, Ordering::Relaxed) {
        return;
    }
    thread::spawn(move || {
        let found = scan_targets(local_targets(), &shared);
        let seen: HashSet<String> = found.iter().map(|(info, _)| info.id.clone()).collect();
        for (info, address) in found {
            apply_found(
                &shared,
                &commands,
                Found::Seen {
                    device_id: info.id,
                    name: info.name,
                    model: info.model,
                    address,
                },
            );
        }
        // 上次扫到、这次没有的算走了
        let gone: Vec<String> = lock(&shared.scanned).difference(&seen).cloned().collect();
        for device_id in gone {
            apply_found(&shared, &commands, Found::Gone { device_id });
        }
        *lock(&shared.scanned) = seen;
        shared.scanning.store(false, Ordering::Relaxed);
    });
}

fn scan_targets(targets: Vec<SocketAddr>, shared: &Shared) -> Vec<(DeviceInfo, SocketAddr)> {
    let queue = Mutex::new(targets);
    let found = Mutex::new(Vec::new());
    let own = shared.identity.device_id();
    thread::scope(|scope| {
        for _ in 0..WORKERS {
            scope.spawn(|| {
                loop {
                    let next = lock(&queue).pop();
                    let Some(address) = next else {
                        return;
                    };
                    if let Ok(info) = probe(address)
                        && info.id != own
                    {
                        lock(&found).push((info, address));
                    }
                }
            });
        }
    });
    found.into_inner().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::neighbours;

    #[test]
    fn neighbours_skip_the_device_itself() {
        let hosts = neighbours(Ipv4Addr::new(192, 168, 0, 187));
        assert_eq!(hosts.len(), 253);
        assert!(!hosts.contains(&Ipv4Addr::new(192, 168, 0, 187)));
        assert!(hosts.contains(&Ipv4Addr::new(192, 168, 0, 190)));
    }
}
