//! 写配置文件：原地改一项再重读，输入法热加载它。

use qingjian_platform::Config;

use super::Settings;

impl Settings {
    pub(super) fn write(&mut self, section: &str, key: &str, value: impl Into<toml_edit::Value>) {
        if let Err(error) = Config::set_value(&self.path, section, key, value) {
            tracing::error!(%error, section, key, "写配置失败");
            return;
        }
        self.reload();
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
            Ok(config) => self.config = config,
            Err(error) => tracing::error!(%error, "重读配置失败"),
        }
    }
}
