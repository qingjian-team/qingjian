//! 已配对设备的名单及其落盘。

mod book;
mod peer;

use std::io;
use std::path::Path;

pub use book::PeerBook;
pub use peer::Peer;

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
    use std::path::PathBuf;

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
