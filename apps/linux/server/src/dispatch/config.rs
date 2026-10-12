//! Linux 首版按键与候选配置。
use qingjian_platform::protocol::KeyModifiers;
use qingjian_platform::{Appearance, AppsConfig, CommitKey, Config, LayoutMode, PreeditMode};

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
    pub appearance: Appearance,

    /// 翻页键对（`[general] page_keys`，上一页 / 下一页）。
    pub page_keys: (char, char),

    /// 英文模式给不给英文候选（`[general] english_candidates`）。
    pub english_candidates: bool,

    /// 记住上次中英（`[general] remember_input_mode`）：切换时写 `last_english`，新会话按它恢复。
    pub remember_input_mode: bool,

    /// 上次中英（`[general] last_english`，`true` 英文）。
    pub last_english: bool,

    /// Shift 大写字母是否加入中文组句。
    pub shift_letter_compose: bool,

    /// 中文模式下不在组句时的标点转全角（`[general] full_width_punctuation`）；状态条可切。
    pub full_width: bool,

    /// 英文模式的那一份（`[general] english_full_width_punctuation`）。
    pub english_full_width: bool,

    /// 组句中敲会转全角的标点（`,` `.` `?` `!` 等，翻页键除外）先把高亮候选上屏、再补该标点
    ///（`nihao,` 一气打完「你好，」）的开关（`[general] punct_commits`），缺省关。
    pub punct_commits: bool,

    /// 拼音 / 双拼组句时哪个键上屏高亮候选（`[general] commit_key`）；注音不受此项影响。
    pub commit_key: CommitKey,

    /// 按应用的设置（`[apps]`），按宿主 exe 名认。
    pub apps: AppsConfig,

    /// 上屏第一 / 第二个译词的修饰键（`[shortcut] translation` / `translation_second`）。
    pub translation_keys: (KeyModifiers, KeyModifiers),

    /// 删候选的修饰键（`[shortcut] delete_candidate`）。
    pub delete_keys: KeyModifiers,
}

impl RouterConfig {
    /// 全局开关开着，且应用在 `[apps] english_candidates_on` 白名单里；没报应用名按不给。
    pub fn english_candidates_in(&self, app: Option<&str>) -> bool {
        self.english_candidates && self.apps.english_candidates_on(app)
    }

    /// 新会话启动时的中英：记住开着且上次是英文时为英文。
    pub fn initial_english(&self) -> bool {
        self.remember_input_mode && self.last_english
    }
}

impl From<&Config> for RouterConfig {
    fn from(config: &Config) -> Self {
        Self {
            page_size: config.general.page_size(),
            preedit: config.general.preedit,
            cloud_slots: 0,
            layout: config.general.layout,
            appearance: config.general.appearance(),
            page_keys: config.general.page_keys(),
            english_candidates: config.general.english_candidates,
            remember_input_mode: config.general.remember_input_mode,
            last_english: config.general.last_english,
            shift_letter_compose: config.general.shift_letter.compose(),
            full_width: config.general.full_width_punctuation,
            english_full_width: config.general.english_full_width_punctuation,
            punct_commits: config.general.punct_commits,
            commit_key: config.general.commit_key,
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
