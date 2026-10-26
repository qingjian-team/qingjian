//! Linux 首版按键与候选配置。
use qingjian_platform::protocol::KeyModifiers;
use qingjian_platform::{
    Appearance, AppsConfig, Config, KeyCombo, LayoutMode, PreeditMode, SwitchKeys,
};

/// Router 要用的配置项，与 macOS 壳的 `Host` 字段对齐。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterConfig {
    /// 每页候选数（`[general] page_size`）。
    pub page_size: usize,

    /// 应用与窗口拼音显示位置。
    pub preedit: PreeditMode,

    /// 云端候选在第一页预留的格数（`[predict] slots`）。
    pub cloud_slots: usize,

    /// 是否请求云端候选词和整句补全（`[predict] enabled`）。
    pub cloud_enabled: bool,

    /// 组句中按需调用 AI 的组合键（`[shortcut] predict`）。
    pub predict: KeyCombo,

    /// 候选排布（`[general] layout`）。
    pub layout: LayoutMode,

    /// 候选窗口外观（`[general] theme`）。
    pub appearance: Appearance,

    /// 翻页键对（`[general] page_keys`，上一页 / 下一页）。
    pub page_keys: (char, char),

    /// 英文模式给不给英文候选（`[general] english_candidates`）。
    pub english_candidates: bool,

    /// 内置英文模式总开关（`[general] english_mode`）：关掉后切换键不再切、会话固定中文。
    pub english_mode: bool,

    /// 新会话从英文模式开始（`[general] default_english`）。
    pub default_english: bool,

    /// 中英切换键（`[shortcut] switch_mode`）。
    pub switch_mode: SwitchKeys,

    /// Shift 大写字母是否加入中文组句。
    pub shift_letter_compose: bool,

    /// 中文模式下不在组句时的标点转全角（`[general] full_width_punctuation`）；状态条可切。
    pub full_width: bool,

    /// 英文模式的那一份（`[general] english_full_width_punctuation`）。
    pub english_full_width: bool,

    /// 组句中敲会转全角的标点（`,` `.` `?` `!` 等，翻页键除外）先把高亮候选上屏、再补该标点
    ///（`nihao,` 一气打完「你好，」）的开关（`[general] punct_commits`），缺省关。
    pub punct_commits: bool,

    /// 按应用的设置（`[apps]`），按宿主 exe 名认。
    pub apps: AppsConfig,

    /// 上屏第一 / 第二个译词的修饰键（`[shortcut] translation` / `translation_second`）。
    pub translation_keys: (KeyModifiers, KeyModifiers),

    /// 翻译应用选区的修饰键与字母（`[shortcut] translate_selection`）。
    pub translate_selection: KeyCombo,

    /// 译词旁显示词性（`[general] translation_pos`）。
    pub translation_pos: bool,
}

impl RouterConfig {
    /// 全局开关开着，且应用在 `[apps] english_candidates_on` 白名单里；没报应用名按不给。
    pub fn english_candidates_in(&self, app: Option<&str>) -> bool {
        self.english_candidates && self.apps.english_candidates_on(app)
    }

    /// 实际生效的切换键：英文模式关着时一个都不认。
    pub fn switch_keys(&self) -> SwitchKeys {
        if self.english_mode {
            self.switch_mode
        } else {
            SwitchKeys::NONE
        }
    }

    /// 新会话的初始模式。
    pub fn starts_english(&self) -> bool {
        self.english_mode && self.default_english
    }
}

impl From<&Config> for RouterConfig {
    fn from(config: &Config) -> Self {
        Self {
            page_size: config.general.page_size(),
            preedit: config.general.preedit,
            cloud_slots: if config.predict.enabled {
                config.predict.slots
            } else {
                0
            },
            cloud_enabled: config.predict.enabled,
            predict: config.shortcut.predict,
            layout: config.general.layout,
            appearance: config.general.appearance(),
            page_keys: config.general.page_keys(),
            english_candidates: config.general.english_candidates,
            english_mode: config.general.english_mode,
            default_english: config.general.default_english,
            switch_mode: config.shortcut.switch_mode,
            shift_letter_compose: config.general.shift_letter.compose(),
            full_width: config.general.full_width_punctuation,
            english_full_width: config.general.english_full_width_punctuation,
            punct_commits: config.general.punct_commits,
            apps: config.apps.clone(),
            translation_keys: {
                let (first, second) = config.shortcut.translation_keys();
                (first.into(), second.into())
            },
            translate_selection: config.shortcut.translate_selection,
            translation_pos: config.general.translation_pos,
        }
    }
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self::from(&Config::default())
    }
}
