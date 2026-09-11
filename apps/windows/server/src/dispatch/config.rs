use qingjian_platform::{Config, LayoutMode, ThemeMode};

/// Router 从 `config.toml` 里要用的那几项，与 macOS 壳的 `Host` 字段对齐。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterConfig {
    /// 每页候选数（`[general] page_size`）。
    pub page_size: usize,

    /// 云端候选在第一页预留的格数（`[predict] slots`）。
    pub cloud_slots: usize,

    /// 候选排布（`[general] layout`），随每帧下发给 DLL。
    pub layout: LayoutMode,

    /// 候选窗口外观（`[general] theme`），随每帧下发；`System` 由 DLL 解析。
    pub theme: ThemeMode,

    /// 翻页键对（`[general] page_keys`，上一页 / 下一页）。
    pub page_keys: (char, char),
}

impl From<&Config> for RouterConfig {
    fn from(config: &Config) -> Self {
        Self {
            page_size: config.general.page_size(),
            cloud_slots: config.predict.slots,
            layout: config.general.layout,
            theme: config.general.theme,
            page_keys: config.general.page_keys(),
        }
    }
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self::from(&Config::default())
    }
}
