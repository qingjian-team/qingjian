//! 清单里的一个模型：卡片上写什么，各运行方式的包从哪个 Release 取，装到哪。

use std::path::{Path, PathBuf};

use qingjian_platform::LocalModel;
use serde::Deserialize;

use crate::i18n::{tr, tr_args};
use crate::jobs::dict_index::Author;
use crate::jobs::download::{Install, Item, Repo, Target, downloads_dir};

/// 跑模型的那套代码。认不得的（以后的新引擎）不列出来。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    Zhiyi,
    Hanzhang,
    #[serde(other)]
    Unknown,
}

impl Engine {
    /// 这一版输入法认得的最新模型格式；清单里写得更大的列出来但不给下，提示先更新输入法。
    fn supported_format(self) -> u16 {
        match self {
            // 1 是 1.4V2-Long-L12，2 是 zhen-feather（候选紧凑排、前缀掩码、选词整句共用一个头）
            Self::Zhiyi => 2,
            Self::Hanzhang => 1,
            Self::Unknown => 0,
        }
    }

    /// 配置文件 `[model] name` 里对应的那个。
    pub fn local_model(self) -> Option<LocalModel> {
        match self {
            Self::Zhiyi => Some(LocalModel::Zhiyi),
            Self::Hanzhang => Some(LocalModel::Tongbian),
            Self::Unknown => None,
        }
    }
}

/// 模型包是给哪种运行方式的。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Runtime {
    /// safetensors 与 `.qjm`，candle 跑。
    Candle,

    /// ONNX Runtime 跑。
    Onnx,

    #[serde(other)]
    Unknown,
}

impl Engine {
    /// 这台机器上这套代码用哪种包：Windows 的知意走 ONNX Runtime（GPU 用 DirectML），别的都走 candle（含章在 Windows 上也是）。
    fn runtime(self) -> Runtime {
        if cfg!(windows) && self == Self::Zhiyi {
            Runtime::Onnx
        } else {
            Runtime::Candle
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct Variant {
    runtime: Runtime,

    /// 模型仓库的 Release 标签。
    tag: String,

    /// 压缩包里那一层目录。
    archive_dir: String,

    /// 拼好的整个压缩包的大小。
    bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ModelEntry {
    /// 装到下载目录的 `models/<id>/`，知意的配置 `[model] zhiyi_model` 也记它。
    pub id: String,

    /// 卡片标题：上游的型号原文。
    pub name: String,

    pub engine: Engine,

    engine_format: u16,

    pub version: String,

    #[serde(default)]
    pub description: String,

    pub license: String,

    #[serde(default)]
    pub author: Author,

    /// 卡片上标「推荐」；只是标签，不替用户换模型。
    #[serde(default)]
    pub recommended: bool,

    /// 弃用了：不再给下载，装了的照样能用。
    #[serde(default)]
    pub deprecated: bool,

    variants: Vec<Variant>,
}

impl ModelEntry {
    /// 这一版输入法能不能列出它：引擎认得、有这台机器能跑的包、id 能当目录名。
    pub(super) fn listed(&self) -> bool {
        self.engine.local_model().is_some() && self.variant().is_some() && plain_id(&self.id)
    }

    /// 格式比这一版输入法新：列出来但不能下，提示先更新输入法。
    pub fn needs_newer_app(&self) -> bool {
        self.engine_format > self.engine.supported_format()
    }

    fn variant(&self) -> Option<&Variant> {
        self.variants
            .iter()
            .find(|variant| variant.runtime == self.engine.runtime())
    }

    /// 这台机器要下的包有多大。
    pub fn bytes(&self) -> u64 {
        self.variant().map_or(0, |variant| variant.bytes)
    }

    pub fn target(&self) -> Option<Target> {
        let variant = self.variant()?;
        Some(Target {
            item: Item::Model(self.id.clone()),
            repo: Repo::Models,
            tag: variant.tag.clone(),
            install: Install::Archive {
                archive_dir: variant.archive_dir.clone(),
                install_dir: format!("models/{}", self.id),
            },
        })
    }

    /// 装在下载目录 `downloads` 下的哪里，与输入法读的位置一致。
    pub fn install_path(&self, downloads: &Path) -> PathBuf {
        downloads.join("models").join(&self.id)
    }

    /// 装好了没有：下载是解到临时目录、整个改名换上的，目录在就是装全了。
    pub fn installed(&self) -> bool {
        downloads_dir().is_some_and(|dir| self.install_path(&dir).is_dir())
    }

    /// 删掉装好的整个模型目录。
    pub fn remove(&self) -> Result<(), String> {
        let dir = downloads_dir().ok_or(tr("common.no_data_dir"))?;
        std::fs::remove_dir_all(self.install_path(&dir))
            .map_err(|error| tr_args("download.cant_remove", &[("error", &error)]))
    }
}

/// 能直接当一段目录名：只有字母、数字与 `-_.`，不全是点。
fn plain_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        && id.chars().any(|c| c != '.')
}
