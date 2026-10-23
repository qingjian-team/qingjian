//! 下载清单与安装位置。权重不打进可执行文件，下载必须通过摘要核对。

use std::path::Path;
use std::sync::OnceLock;

use crate::Package;

pub const MODEL_DIR: &str = "models/offline-translation";

pub fn packages() -> &'static [Package] {
    static PACKAGES: OnceLock<Vec<Package>> = OnceLock::new();
    PACKAGES.get_or_init(|| {
        serde_json::from_str(include_str!("catalog.json")).expect("内置翻译模型清单必须有效")
    })
}

pub fn installed(dir: &Path) -> bool {
    packages().iter().all(|pack| {
        pack.files.iter().all(|asset| {
            dir.join(&pack.direction)
                .join(&asset.name)
                .metadata()
                .is_ok_and(|metadata| metadata.is_file() && metadata.len() == asset.bytes)
        })
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn pinned_catalog_has_both_directions_and_digests() {
        let packs = super::packages();
        assert_eq!(packs.len(), 2);
        for pack in packs {
            assert_eq!(pack.revision.len(), 40);
            assert!(matches!(pack.direction.as_str(), "zh-en" | "en-zh"));
            assert_eq!(pack.files.len(), 5);
            for asset in &pack.files {
                assert_eq!(asset.sha256.len(), 64);
                assert!(asset.url.contains(&pack.revision));
                assert!(!asset.name.contains(['/', '\\']));
            }
        }
    }
}
