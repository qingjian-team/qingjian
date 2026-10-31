//! 配置来源的文件系统快照；不读取、散列或保存配置正文。

use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(crate) struct ConfigSource {
    pub(crate) path: Option<PathBuf>,

    pub(crate) resolved_path: Option<PathBuf>,

    pub(crate) bytes: Option<u64>,

    pub(crate) modified_unix_ms: Option<u128>,
}

impl ConfigSource {
    pub(crate) fn inspect(path: Option<&Path>) -> Self {
        let metadata = path.and_then(|path| std::fs::metadata(path).ok());
        Self {
            path: path.map(Path::to_path_buf),
            resolved_path: path.and_then(|path| std::fs::canonicalize(path).ok()),
            bytes: metadata.as_ref().map(std::fs::Metadata::len),
            modified_unix_ms: metadata
                .and_then(|metadata| metadata.modified().ok())
                .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_millis()),
        }
    }

    pub(crate) fn print(&self) {
        println!(
            "config_path={:?} config_resolved_path={:?} config_bytes={:?} config_modified_unix_ms={:?}",
            self.path, self.resolved_path, self.bytes, self.modified_unix_ms
        );
    }
}

#[cfg(test)]
mod tests {
    use super::ConfigSource;

    #[test]
    fn source_metadata_distinguishes_missing_and_existing_files() {
        let path = std::env::temp_dir().join(format!(
            "qingjian-config-source-{}-{}.toml",
            std::process::id(),
            super::super::timestamp_unix_ms()
        ));
        let missing = ConfigSource::inspect(Some(&path));
        assert_eq!(missing.path.as_deref(), Some(path.as_path()));
        assert!(missing.resolved_path.is_none());
        assert!(missing.bytes.is_none());
        assert!(missing.modified_unix_ms.is_none());

        std::fs::write(&path, b"enabled=true\n").unwrap();
        let found = ConfigSource::inspect(Some(&path));
        assert_eq!(
            found.resolved_path,
            Some(std::fs::canonicalize(&path).unwrap())
        );
        assert_eq!(found.bytes, Some(13));
        assert_eq!(
            found.modified_unix_ms,
            Some(
                std::fs::metadata(&path)
                    .unwrap()
                    .modified()
                    .unwrap()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
            )
        );
        std::fs::remove_file(path).unwrap();
    }
}
