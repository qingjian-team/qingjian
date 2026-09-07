//! 词库管理：随包领域词库开关，用户导入词库的导入 / 移除 / 开关，重新加载。

use super::init::{dictionary_info, string_array, toggle_membership};
use super::*;

impl Host {
    /// 「词库」页的列表：随包领域词库在前、用户目录 `dicts/` 里的在后（含关掉的）。
    pub(super) fn dictionary_infos(&self) -> Vec<DictionaryInfo> {
        let config = &self.settings.config().dictionaries;
        let mut infos = Vec::new();
        if let Some(dir) = paths::bundled_dicts_dir() {
            for (stem, path) in extra_dictionaries::list(&dir) {
                let enabled = config.is_domain_enabled(&stem);
                infos.push(dictionary_info(stem, path, true, enabled));
            }
        }
        if let Some(dir) = paths::dicts_dir() {
            for (stem, path) in extra_dictionaries::list(&dir) {
                let enabled = config.is_enabled(&stem);
                infos.push(dictionary_info(stem, path, false, enabled));
            }
        }
        infos
    }

    /// 导入一本词库（TSV / Rime yaml / .qj）到 `dicts/`，然后重新装配。
    pub fn import_dictionary(&mut self, source: &std::path::Path) {
        let Some(dir) = paths::dicts_dir() else {
            return;
        };
        match qingjian_dictionary::import::import(source, &dir) {
            Ok(imported) => {
                tracing::info!(name = %imported.name, entries = imported.entries, path = %imported.path.display(), "词库已导入");
                // 之前关掉过同名词库的，导入后自动打开
                let stem = imported
                    .path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_owned();
                self.set_dictionary_enabled(&stem, true);
                self.reload_dictionaries();
                self.apply_config(false);
            }
            Err(error) => tracing::warn!(source = %source.display(), %error, "词库导入失败"),
        }
    }

    /// 移除一本导入的词库：文件挪到 `dicts/removed/`（可手工找回），配置里的开关项一并清掉。随包词库不能移除。
    pub fn remove_dictionary(&mut self, index: usize) {
        let Some(info) = self.dictionary_list.get(index).cloned() else {
            return;
        };
        if info.builtin {
            return;
        }
        let Some(dir) = paths::dicts_dir() else {
            return;
        };
        let removed = dir.join("removed");
        let moved = std::fs::create_dir_all(&removed).and_then(|()| {
            std::fs::rename(
                &info.path,
                removed.join(info.path.file_name().unwrap_or_default()),
            )
        });
        match moved {
            Ok(()) => tracing::info!(name = %info.name, "词库已移除（挪到 dicts/removed/）"),
            Err(error) => tracing::warn!(%error, path = %info.path.display(), "移除词库失败"),
        }
        self.set_dictionary_enabled(&info.stem, true);
        self.reload_dictionaries();
        self.apply_config(false);
    }

    /// 改 `[dictionaries] disabled`：`on` 为真从列表里去掉，否则加进去。
    pub fn set_dictionary_enabled(&mut self, stem: &str, on: bool) {
        let disabled = self.settings.config().dictionaries.disabled.clone();
        if let Some(disabled) = toggle_membership(disabled, stem, !on) {
            self.settings.set_value(
                "dictionaries",
                "disabled",
                toml_edit::Value::Array(string_array(&disabled)),
            );
        }
    }

    /// 改 `[dictionaries] domains`（随包领域词库）：`on` 为真加进列表，否则去掉。
    pub fn set_domain_enabled(&mut self, stem: &str, on: bool) {
        let domains = self.settings.config().dictionaries.domains.clone();
        if let Some(domains) = toggle_membership(domains, stem, on) {
            self.settings.set_value(
                "dictionaries",
                "domains",
                toml_edit::Value::Array(string_array(&domains)),
            );
        }
    }

    /// 按当前配置重新加载 `dicts/` 目录（导入、移除、开关之后）。
    pub fn reload_dictionaries(&mut self) {
        let config = self.settings.config().dictionaries.clone();
        let loaded = extra_dictionaries::load(
            paths::bundled_dicts_dir().as_deref(),
            paths::dicts_dir().as_deref(),
            &config,
        );
        tracing::info!(count = loaded.len(), "附加词库已装配");
        self.engine.set_extra_dictionaries(loaded);
        self.applied_dictionaries = config;
    }
}
