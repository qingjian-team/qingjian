//! 固定版本下载、逐文件 SHA256 核对、原子换入；失败不覆盖已安装模型。

use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use super::shared::Shared;
use crate::jobs::model_download::Stage;

pub(super) fn run(user: &Path, shared: &Shared) -> Result<(), String> {
    let models = user.join("models");
    std::fs::create_dir_all(&models).map_err(describe)?;
    let _lock = download_lock(&models)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(describe)?;
    let work = user.join("models/.offline-translation-download");
    recover(&work, &user.join(qingjian_offline::MODEL_DIR))?;
    if work.exists() {
        std::fs::remove_dir_all(&work).map_err(describe)?;
    }
    std::fs::create_dir_all(&work).map_err(describe)?;
    let result = runtime.block_on(download(&work, shared)).and_then(|()| {
        if shared.cancelled() {
            return Err("已取消下载".into());
        }
        shared.set(Stage::Installing);
        install(
            &work.join("new"),
            &user.join(qingjian_offline::MODEL_DIR),
            &work.join("old"),
        )
    });
    // 安装回滚失败时保留备份，避免自动清理唯一可恢复的数据。
    if !work.join("old").exists() {
        let _ = std::fs::remove_dir_all(&work);
    }
    result
}

fn download_lock(models: &Path) -> Result<File, String> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(models.join(".offline-translation.lock"))
        .map_err(describe)?;
    file.try_lock()
        .map_err(|_| "另一个设置窗口正在下载或安装翻译模型，请稍后再试".to_owned())?;
    Ok(file)
}

async fn download(work: &Path, shared: &Shared) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(30))
        .build()
        .map_err(describe)?;
    let total = qingjian_offline::packages()
        .iter()
        .flat_map(|p| &p.files)
        .map(|f| f.bytes)
        .sum();
    let mut done = 0;
    let mut report = Instant::now();
    for pack in qingjian_offline::packages() {
        let dir = work.join("new").join(&pack.direction);
        std::fs::create_dir_all(&dir).map_err(describe)?;
        for asset in &pack.files {
            if shared.cancelled() {
                return Err("已取消下载".into());
            }
            let mut response = client
                .get(&asset.url)
                .send()
                .await
                .map_err(describe)?
                .error_for_status()
                .map_err(describe)?;
            let mut file = File::create(dir.join(&asset.name)).map_err(describe)?;
            let mut digest = Sha256::new();
            let mut bytes = 0;
            while let Some(block) = response.chunk().await.map_err(describe)? {
                if shared.cancelled() {
                    return Err("已取消下载".into());
                }
                bytes += block.len() as u64;
                if bytes > asset.bytes {
                    return Err("模型文件长度不符合清单".into());
                }
                file.write_all(&block).map_err(describe)?;
                digest.update(&block);
                done += block.len() as u64;
                if report.elapsed() >= Duration::from_millis(100) {
                    shared.set(Stage::Downloading { done, total });
                    report = Instant::now();
                }
            }
            let hash: String = digest
                .finalize()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            if bytes != asset.bytes || hash != asset.sha256 {
                return Err("模型文件校验失败，请重试".into());
            }
            file.sync_all().map_err(describe)?;
        }
    }
    shared.set(Stage::Downloading { done, total });
    Ok(())
}

fn install(source: &Path, target: &Path, backup: &Path) -> Result<(), String> {
    if target.exists() {
        std::fs::rename(target, backup).map_err(describe)?;
    }
    if let Err(error) = std::fs::rename(source, target) {
        if backup.exists() {
            std::fs::rename(backup, target).map_err(describe)?;
        }
        return Err(describe(error));
    }
    if backup.exists() {
        std::fs::remove_dir_all(backup).map_err(describe)?;
    }
    Ok(())
}

fn recover(work: &Path, target: &Path) -> Result<(), String> {
    let backup = work.join("old");
    if !backup.exists() {
        return Ok(());
    }
    if target.exists() {
        return Err(format!(
            "上次安装备份仍在 {}，请先恢复或核对模型后再下载",
            backup.display()
        ));
    }
    std::fs::rename(backup, target).map_err(describe)
}

fn describe(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    #[test]
    fn concurrent_download_cannot_remove_another_downloads_work() {
        let dir =
            std::env::temp_dir().join(format!("qingjian-translation-lock-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let guard = super::download_lock(&dir).unwrap();
        assert!(super::download_lock(&dir).is_err());
        drop(guard);
        assert!(super::download_lock(&dir).is_ok());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn retry_recovers_a_backup_before_cleaning_the_work_directory() {
        let dir = std::env::temp_dir().join(format!(
            "qingjian-translation-recovery-{}",
            std::process::id()
        ));
        let work = dir.join("work");
        let backup = work.join("old");
        std::fs::create_dir_all(&backup).unwrap();
        std::fs::write(backup.join("previous"), b"valid").unwrap();
        let target = dir.join("model");
        super::recover(&work, &target).unwrap();
        assert_eq!(std::fs::read(target.join("previous")).unwrap(), b"valid");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    #[ignore = "downloads real public model files"]
    fn downloads_and_verifies_real_translation_models() {
        let dir = std::env::var_os("QINGJIAN_NATIVE_DOWNLOAD_TEST_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
                    "../../target/native-download-check-{}",
                    std::process::id()
                ))
            });
        assert!(dir.is_absolute());
        assert!(
            !dir.join(qingjian_offline::MODEL_DIR).exists(),
            "use a new test profile"
        );
        std::fs::create_dir_all(&dir).unwrap();
        let shared = super::Shared::new(None);
        super::run(&dir, &shared).unwrap();
        assert!(qingjian_offline::installed(
            &dir.join(qingjian_offline::MODEL_DIR)
        ));
        println!(
            "verified models: {}",
            dir.join(qingjian_offline::MODEL_DIR).display()
        );
    }

    #[test]
    fn failed_install_restores_the_previous_model() {
        let dir = std::env::temp_dir().join(format!(
            "qingjian-translation-rollback-{}",
            std::process::id()
        ));
        let target = dir.join("model");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("previous"), b"valid").unwrap();
        assert!(super::install(&dir.join("missing"), &target, &dir.join("backup")).is_err());
        assert_eq!(std::fs::read(target.join("previous")).unwrap(), b"valid");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
