//! 原生雾凇全拼的数据位置；不提供 Rime 运行库或按键配置。

use qingjian_core::Engine;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RimeIceConfig {
    pub enabled: bool,

    /// 用户自行下载的雾凇目录，必须是绝对路径。
    pub data_dir: PathBuf,
}

impl RimeIceConfig {
    pub fn apply(&self, engine: &mut Engine, cache: &Path) -> std::io::Result<()> {
        if !self.enabled {
            engine.disable_rime_ice();
            return Ok(());
        }
        if !self.data_dir.is_absolute() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "rime_ice.data_dir must be an absolute path",
            ));
        }
        engine.load_rime_ice(&self.data_dir, cache)
    }
}
