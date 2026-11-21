//! 已配对设备的名单及其落盘。

mod book;
mod peer;

use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

pub use book::PeerBook;
pub use peer::Peer;

/// 先写临时文件再替换。文件放在应用私有目录，同时把文件本身收紧到 0600：
/// 目录权限配错（或从别处搬来一个 0755 的目录）时，私钥也不会被同机其他用户读走。
pub(crate) fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = path.with_extension(format!("tmp-{}-{sequence}", std::process::id()));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let outcome = (|| {
        let mut file = options.open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    })();
    if outcome.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    outcome
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

    /// 私钥文件不能只靠目录权限：目录配成 0755 时，文件本身也要是 0600。
    #[cfg(unix)]
    #[test]
    fn private_files_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = folder("private");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("identity");
        write_private(&path, b"secret").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
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
