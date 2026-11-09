//! 下完、核对过之后怎么装：模型是 tar.gz 里的一个目录，词库是一个 `.qjz`。中途失败都不动原来装着的。

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use qingjian_format::{Kind, qjz};

use crate::i18n::{tr, tr_args};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Install {
    /// 模型：压缩包里的 `archive_dir` 整个换进下载目录（`downloads/`）下的 `install_dir`（与输入法读的位置一致）。
    Archive {
        archive_dir: &'static str,

        install_dir: &'static str,
    },

    /// 词库：`.qjz` 解成下载目录（`downloads/`）下的 `dest`（相对路径，如 `dicts/domain-medicine.qj`），解出来要是 `kind`。
    Qjz { dest: String, kind: Kind },
}

impl Install {
    /// 装到的目录的上一级：下载的临时目录放这里。
    pub(super) fn parent(&self, downloads: &Path) -> PathBuf {
        let target = self.target(downloads);
        target.parent().map_or(target.clone(), Path::to_path_buf)
    }

    fn target(&self, downloads: &Path) -> PathBuf {
        match self {
            Self::Archive { install_dir, .. } => downloads.join(install_dir),
            Self::Qjz { dest, .. } => downloads.join(dest),
        }
    }

    /// 把下好的 `archive` 装进下载目录；`work` 是这次下载的临时目录。
    pub(super) fn run(&self, archive: &Path, work: &Path, downloads: &Path) -> Result<(), String> {
        let target = self.target(downloads);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| tr_args("download.cant_create_model_dir", &[("error", &error)]))?;
        }
        match self {
            Self::Archive { archive_dir, .. } => {
                unpack_archive(archive, archive_dir, work, &target)
            }
            // unpack 先写旁边的 .qj.part、校验过种类才改名换上
            Self::Qjz { kind, .. } => qjz::unpack(archive, &target, *kind)
                .map(|_| ())
                .map_err(|error| tr_args("download.unpack_failed", &[("error", &error)])),
        }
    }
}

/// 解到临时目录，再把旧的模型目录换掉。
fn unpack_archive(
    archive: &Path,
    archive_dir: &str,
    work: &Path,
    target: &Path,
) -> Result<(), String> {
    let extracted = work.join("extracted");
    let file =
        File::open(archive).map_err(|error| tr_args("download.cant_open", &[("error", &error)]))?;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(BufReader::new(file)));
    // unpack 不放行绝对路径与 `..`，解不出包外
    tar.unpack(&extracted)
        .map_err(|error| tr_args("download.unpack_failed", &[("error", &error)]))?;
    let source = extracted.join(archive_dir);
    if !source.is_dir() {
        return Err(tr("download.no_model_dir").to_owned());
    }
    let old = work.join("old");
    if target.exists() {
        std::fs::rename(target, &old)
            .map_err(|error| tr_args("download.cant_move_old", &[("error", &error)]))?;
    }
    if let Err(error) = std::fs::rename(&source, target) {
        let _ = std::fs::rename(&old, target);
        return Err(tr_args("download.cant_install", &[("error", &error)]));
    }
    Ok(())
}
