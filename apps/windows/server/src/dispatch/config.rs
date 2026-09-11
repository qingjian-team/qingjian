use qingjian_platform::protocol::KeyModifiers;
use qingjian_platform::{AppsConfig, Config, LayoutMode, ThemeMode};

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

    /// 英文模式（Caps Lock 亮）给不给英文候选（`[general] english_candidates`）；关着就是纯直通。
    pub english_candidates: bool,

    /// 按应用的设置（`[apps]`）：英文模式不给候选的应用，按宿主进程的 exe 文件名认。
    pub apps: AppsConfig,

    /// 数字键配这两组修饰键上屏候选的第一 / 第二个译词（`[shortcut] translation` / `translation_second`，缺省 Alt 与 Shift+Alt）。
    pub translation_keys: (KeyModifiers, KeyModifiers),

    /// 数字键配这组修饰键删掉候选（`[shortcut] delete_candidate`，缺省 Shift）。
    pub delete_keys: KeyModifiers,
}

impl RouterConfig {
    /// 这个应用里英文模式给不给候选：全局开关开着，且应用不在 `[apps] english_candidates_off` 里
    /// （与 macOS 壳的 `Host::english_candidates_in` 对齐）。`app` 是会话开时报的 exe 名，没报按不关。
    pub fn english_candidates_in(&self, app: Option<&str>) -> bool {
        self.english_candidates && !app.is_some_and(|app| self.apps.english_candidates_off(app))
    }
}

impl From<&Config> for RouterConfig {
    fn from(config: &Config) -> Self {
        Self {
            page_size: config.general.page_size(),
            cloud_slots: config.predict.slots,
            layout: config.general.layout,
            theme: config.general.theme,
            page_keys: config.general.page_keys(),
            english_candidates: config.general.english_candidates,
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
