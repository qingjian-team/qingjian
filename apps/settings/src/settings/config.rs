//! 写配置文件：原地改一项再重读，输入法热加载它。

use qingjian_platform::{Config, LocalModel};

use crate::jobs::model_index::ModelEntry;

use super::Settings;

impl Settings {
    pub(super) fn write(&mut self, section: &str, key: &str, value: impl Into<toml_edit::Value>) {
        if let Err(error) = Config::set_value(&self.path, section, key, value) {
            tracing::error!(%error, section, key, "写配置失败");
            return;
        }
        self.reload();
    }

    /// 开清单里的一个模型（写 `name`，知意再写用哪一版，最后打开 `enabled`），`None` 是关掉、不用模型（选的留着）。
    pub(super) fn use_model(&mut self, entry: Option<&ModelEntry>) {
        let model = entry.and_then(|entry| Some((entry, entry.engine.local_model()?)));
        if let Some((entry, model)) = model {
            self.write("model", "name", model.key());
            if model == LocalModel::Zhiyi {
                self.write("model", "zhiyi_model", entry.id.as_str());
            }
        }
        self.write("model", "enabled", model.is_some());
    }

    /// 清单里的这个模型是不是正开着。
    pub fn model_on(&self, entry: &ModelEntry) -> bool {
        let model = &self.config.model;
        model.enabled
            && entry.engine.local_model() == Some(model.name)
            && (model.name != LocalModel::Zhiyi || model.zhiyi_model == entry.id)
    }

    pub(super) fn write_list(&mut self, section: &str, key: &str, values: &[String]) {
        if let Err(error) = Config::set_array(&self.path, section, key, values) {
            tracing::error!(%error, section, key, "写配置失败");
            return;
        }
        self.reload();
    }

    pub(super) fn reload(&mut self) {
        match Config::load(&self.path) {
            Ok(config) => {
                self.config = config;
                self.config_error = None;
            }
            Err(error) => {
                tracing::error!(%error, "重读配置失败");
                self.config_error = Some(error.to_string());
            }
        }
    }
}
