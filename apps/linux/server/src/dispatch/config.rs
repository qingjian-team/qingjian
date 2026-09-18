//! Linux 首版按键与候选配置。
use qingjian_platform::protocol::KeyModifiers;
use qingjian_platform::{AppsConfig, Config, LayoutMode, PreeditMode, ThemeMode};

/// Router 要用的配置项，与 macOS 壳的 `Host` 字段对齐。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterConfig {
    /// 每页候选数（`[general] page_size`）。
    pub page_size: usize,

    /// 应用与窗口拼音显示位置。
    pub preedit: PreeditMode,

    /// 云端候选在第一页预留的格数（`[predict] slots`）。
    pub cloud_slots: usize,

    /// 候选排布（`[general] layout`）。
    pub layout: LayoutMode,

    /// 候选窗口外观（`[general] theme`）。
    pub theme: ThemeMode,

    /// 翻页键对（`[general] page_keys`，上一页 / 下一页）。
    pub page_keys: (char, char),

    /// 英文模式给不给英文候选（`[general] english_candidates`）。
    pub english_candidates: bool,

    /// 中文模式下不在组句时的标点转全角（`[general] full_width_punctuation`）；状态条可切。
    pub full_width: bool,

    /// 英文模式的那一份（`[general] english_full_width_punctuation`）。
    pub english_full_width: bool,

    /// 按应用的设置（`[apps]`），按宿主 exe 名认。
    pub apps: AppsConfig,

    /// 上屏第一 / 第二个译词的修饰键（`[shortcut] translation` / `translation_second`）。
    pub translation_keys: (KeyModifiers, KeyModifiers),

    /// 删候选的修饰键（`[shortcut] delete_candidate`）。
    pub delete_keys: KeyModifiers,
}

impl RouterConfig {
    /// 全局开关开着，且应用不在 `[apps] english_candidates_off` 里；没报 exe 名按不关。
    pub fn english_candidates_in(&self, app: Option<&str>) -> bool {
        self.english_candidates && !app.is_some_and(|app| self.apps.english_candidates_off(app))
    }
}

impl From<&Config> for RouterConfig {
    fn from(config: &Config) -> Self {
        Self {
            page_size: config.general.page_size(),
            preedit: config.general.preedit,
            cloud_slots: 0,
            layout: config.general.layout,
            theme: config.general.theme,
            page_keys: config.general.page_keys(),
            english_candidates: config.general.english_candidates,
            full_width: config.general.full_width_punctuation,
            english_full_width: config.general.english_full_width_punctuation,
            apps: config.apps.clone(),
            translation_keys: {
                let (first, second) = config.shortcut.translation_keys();
                (first.into(), second.into())
            },
            delete_keys: config.shortcut.delete_keys().into(),
        }
    }
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self::from(&Config::default())
    }
}
