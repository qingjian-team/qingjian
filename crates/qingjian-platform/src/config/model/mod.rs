mod local_model;
mod zhiyi_tier;

pub use local_model::LocalModel;
pub use zhiyi_tier::ZhiyiTier;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// 知意缺省用的那一版：模型仓库清单里的 id，也是下载目录 `models/` 下的子目录名。
const DEFAULT_ZHIYI_MODEL: &str = "zhiyi";

/// 配置文件 `[model]` 分节：本地整句模型的开关。
///
/// 本地模型在本机给整句候选重新排序或直接造句，全程离线、不联网，与云联想互不影响（本地先出、云端到了另占它自己的格）。
/// 选的模型还没下载时不用模型（Linux 随包带通变）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LocalModelConfig {
    /// 开着就加载模型、给整句重排或造句。缺省关：只按词库与语言模型排序，最快也最省电。
    pub enabled: bool,

    /// 用哪个模型。
    pub name: LocalModel,

    /// 知意用哪一版：模型仓库清单里的 id（`zhiyi`、`zhiyi-feather`），装在下载目录的 `models/<id>/`。用 [`Self::zhiyi_dir`] 取目录。
    pub zhiyi_model: String,

    /// 知意的档位，别的模型不看。
    pub zhiyi_tier: ZhiyiTier,

    /// 应用读不到光标前文时，这个应用里多少秒内上屏的字还给模型当前文；0 表示一直算。
    pub context_memory_seconds: u64,
}

impl Default for LocalModelConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            name: LocalModel::default(),
            zhiyi_model: DEFAULT_ZHIYI_MODEL.to_owned(),
            zhiyi_tier: ZhiyiTier::default(),
            context_memory_seconds: qingjian_core::DEFAULT_FRESH_SECONDS,
        }
    }
}

impl LocalModelConfig {
    /// 两份配置要不要加载同一个模型：开关、选的模型一样，选知意时版本也一样。档位与前文记多久不算，当场就能改。
    pub fn loads_same_model(&self, other: &Self) -> bool {
        self.enabled == other.enabled
            && self.name == other.name
            && (self.name != LocalModel::Zhiyi || self.zhiyi_model == other.zhiyi_model)
    }

    /// 选的那一版知意在下载的模型目录 `models` 下的位置；id 不是单独一段名字（带斜杠、`..`、空的）时退回缺省那一版，配置写坏了也读不到模型目录外面。
    pub fn zhiyi_dir(&self, models: &Path) -> PathBuf {
        let id = self.zhiyi_model.as_str();
        let plain = !id.is_empty()
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
            && id.chars().any(|c| c != '.');
        models.join(if plain { id } else { DEFAULT_ZHIYI_MODEL })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switching_zhiyi_versions_reloads_only_while_zhiyi_is_chosen() {
        let zhiyi = LocalModelConfig {
            enabled: true,
            name: LocalModel::Zhiyi,
            ..LocalModelConfig::default()
        };
        let feather = LocalModelConfig {
            zhiyi_model: "zhiyi-feather".to_owned(),
            ..zhiyi.clone()
        };
        let tier = LocalModelConfig {
            zhiyi_tier: ZhiyiTier::Accurate,
            ..zhiyi.clone()
        };
        assert!(!zhiyi.loads_same_model(&feather));
        assert!(zhiyi.loads_same_model(&tier));
        let tongbian = |config: &LocalModelConfig| LocalModelConfig {
            name: LocalModel::Tongbian,
            ..config.clone()
        };
        assert!(tongbian(&zhiyi).loads_same_model(&tongbian(&feather)));
    }

    #[test]
    fn zhiyi_dir_stays_inside_the_models_folder() {
        let models = Path::new("models");
        let dir = |id: &str| {
            let config = LocalModelConfig {
                zhiyi_model: id.to_owned(),
                ..LocalModelConfig::default()
            };
            config.zhiyi_dir(models)
        };
        assert_eq!(dir("zhiyi-feather"), models.join("zhiyi-feather"));
        for bad in ["", "..", ".", "../zhiyi", "a/b", "a\\b", "/abs"] {
            assert_eq!(dir(bad), models.join(DEFAULT_ZHIYI_MODEL), "{bad}");
        }
    }
}
