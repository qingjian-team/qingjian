//! [`PeerBook`]：已配对设备的名单，存成一个 JSON 文件。连接时靠它判断对方能不能进。

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::SyncError;

/// 连上的时刻多久落一次盘。
const TOUCH_SAVE_MILLIS: i64 = 60_000;

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

#[derive(Debug, Default)]
pub struct PeerBook {
    peers: Vec<Peer>,

    path: PathBuf,
}

impl PeerBook {
    /// 读名单，文件不存在就是空的。文件坏了报错，不当空名单，免得保存时冲掉配对。
    pub fn open(path: &Path) -> Result<Self, SyncError> {
        let peers = match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|error| SyncError::Storage(error.to_string()))?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(SyncError::Storage(error.to_string())),
        };
        Ok(Self {
            peers,
            path: path.to_owned(),
        })
    }

    pub fn peers(&self) -> &[Peer] {
        &self.peers
    }

    pub fn contains(&self, public: &[u8; 32]) -> bool {
        let id = hex(public);
        self.peers.iter().any(|peer| peer.device_id == id)
    }

    /// 配对成功后记下对方；已有的更新名字和地址。
    pub fn add(
        &mut self,
        public: &[u8; 32],
        name: String,
        last_address: Option<String>,
    ) -> Result<(), SyncError> {
        let device_id = hex(public);
        self.peers.retain(|peer| peer.device_id != device_id);
        self.peers.push(Peer {
            device_id,
            name,
            last_address,
            last_seen: None,
        });
        self.save()
    }

    /// 记一次连上。时刻每分钟才落一次盘，同步频繁时不反复写文件；地址变了马上写。
    pub fn touch(
        &mut self,
        device_id: &str,
        now: i64,
        address: Option<String>,
    ) -> Result<(), SyncError> {
        let Some(peer) = self
            .peers
            .iter_mut()
            .find(|peer| peer.device_id == device_id)
        else {
            return Ok(());
        };
        let moved = address.is_some() && address != peer.last_address;
        let stale = peer
            .last_seen
            .is_none_or(|seen| now - seen >= TOUCH_SAVE_MILLIS);
        if moved {
            peer.last_address = address;
        }
        if moved || stale {
            peer.last_seen = Some(now);
            self.save()?;
        }
        Ok(())
    }

    /// 解除配对，之后对方连不进来。
    pub fn remove(&mut self, device_id: &str) -> Result<(), SyncError> {
        self.peers.retain(|peer| peer.device_id != device_id);
        self.save()
    }

    fn save(&self) -> Result<(), SyncError> {
        let storage = |error: io::Error| SyncError::Storage(error.to_string());
        write_private(&self.path, &serde_json::to_vec_pretty(&self.peers)?).map_err(storage)
    }
}

/// 先写临时文件再替换。文件放在应用私有目录，靠目录权限保护。
pub(crate) fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(&temporary, path)
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::Identity;

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("qj-sync-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn peers_persist_and_can_be_removed() {
        let dir = folder("peers");
        let path = dir.join("peers.json");
        let key = [7_u8; 32];
        let mut book = PeerBook::open(&path).unwrap();
        assert!(!book.contains(&key));
        book.add(&key, "手机".into(), Some("192.168.1.5:4000".into()))
            .unwrap();
        let mut book = PeerBook::open(&path).unwrap();
        assert!(book.contains(&key));
        assert_eq!(book.peers()[0].name, "手机");
        book.remove(&hex(&key)).unwrap();
        assert!(!PeerBook::open(&path).unwrap().contains(&key));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn corrupt_files_are_errors_not_empty() {
        let dir = folder("corrupt");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("peers.json");
        std::fs::write(&path, b"{oops").unwrap();
        assert!(PeerBook::open(&path).is_err());
        let identity = dir.join("identity");
        std::fs::write(&identity, b"short").unwrap();
        assert!(Identity::load_or_create(&identity).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn identity_is_stable_across_loads() {
        let dir = folder("identity");
        let path = dir.join("identity");
        let first = Identity::load_or_create(&path).unwrap();
        let second = Identity::load_or_create(&path).unwrap();
        assert_eq!(first.public, second.public);
        assert_eq!(first.device_id(), second.device_id());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
