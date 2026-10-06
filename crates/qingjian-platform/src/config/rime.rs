//! 可选原生 Rime 后端配置；启用和更改路径后重启输入法服务。
use qingjian_core::RimeOptions;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RimeConfig {
    pub enabled: bool,

    pub library: PathBuf,

    pub shared_data: PathBuf,

    pub user_data: PathBuf,

    pub schema: String,

    pub modules: Vec<String>,
}

impl Default for RimeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            library: PathBuf::new(),
            shared_data: PathBuf::new(),
            user_data: PathBuf::new(),
            schema: "rime_ice".to_owned(),
            modules: Vec::new(),
        }
    }
}

impl RimeConfig {
    /// 显式启用时将路径交给 Core 校验。运行库错误必须报告给用户。
    pub fn options(&self) -> Option<RimeOptions> {
        self.enabled.then(|| RimeOptions {
            library: self.library.clone(),
            shared_data: self.shared_data.clone(),
            user_data: self.user_data.clone(),
            schema: self.schema.clone(),
            modules: self.modules.clone(),
        })
    }
}
