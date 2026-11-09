//! 能在「实验功能」里下载的本地模型：标签与装到哪写死在代码里，主版本变了旧输入法读不了。

use std::path::{Path, PathBuf};

use qingjian_platform::LocalModel;

use super::{Install, Item, Repo, Target, downloads_dir};

use crate::i18n::{tr, tr_args};

/// 一个能在「实验功能」里下载的模型：从模型仓库的哪个 Release 取，解到下载目录的哪里。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pack {
    pub model: LocalModel,

    /// 模型仓库（qingjian-team/qingjian-models）的标签。主版本写死在这里：换了主版本，旧输入法读不了。
    pub tag: &'static str,

    /// 压缩包里那一层目录。
    archive_dir: &'static str,

    /// 解到下载目录（`downloads/`）下的哪里，与输入法读的位置一致。
    install_dir: &'static str,

    /// 有这个文件就算装好了。
    marker: &'static str,

    /// 卡片上写的大小。
    pub size: &'static str,
}

impl Pack {
    /// macOS 用 safetensors 版（candle 跑）。
    #[cfg(not(windows))]
    const ZHIYI: Self = Self {
        model: LocalModel::Zhiyi,
        tag: "zhiyi-v1.0.0",
        archive_dir: "zhiyi",
        install_dir: "models/zhiyi",
        marker: "model.safetensors",
        size: "154 MB",
    };

    /// Windows 用 int8 ONNX 版（ONNX Runtime 跑）。
    #[cfg(windows)]
    const ZHIYI: Self = Self {
        model: LocalModel::Zhiyi,
        tag: "zhiyi-onnx-v1.0.0",
        archive_dir: "zhiyi-onnx",
        install_dir: "models/zhiyi-onnx",
        marker: "laya.int8g.onnx",
        size: "70 MB",
    };

    pub const ALL: [Self; 2] = [
        Self::ZHIYI,
        Self {
            model: LocalModel::Tongbian,
            tag: "hanzhang-tongbian-v1.0.0",
            archive_dir: "hanzhang/tongbian",
            install_dir: "models/hanzhang-tongbian",
            marker: "hanzhang-tongbian-small.qjm",
            size: "43 MB",
        },
    ];

    pub fn of(model: LocalModel) -> Option<Self> {
        Self::ALL.into_iter().find(|pack| pack.model == model)
    }

    pub fn install_path(self, downloads: &Path) -> PathBuf {
        downloads.join(self.install_dir)
    }

    pub fn target(self) -> Target {
        Target {
            item: Item::Model(self.model),
            repo: Repo::Models,
            tag: self.tag.to_owned(),
            install: Install::Archive {
                archive_dir: self.archive_dir,
                install_dir: self.install_dir,
            },
        }
    }

    /// 删掉装好的整个模型目录。
    pub fn remove(self) -> Result<(), String> {
        let dir = downloads_dir().ok_or(tr("common.no_data_dir"))?;
        std::fs::remove_dir_all(self.install_path(&dir))
            .map_err(|error| tr_args("download.cant_remove", &[("error", &error)]))
    }

    pub fn installed(self) -> bool {
        downloads_dir().is_some_and(|dir| self.install_path(&dir).join(self.marker).is_file())
    }
}
