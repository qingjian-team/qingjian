mod value;

pub use value::SettingValue;

use objc2_foundation::NSInteger;
use qingjian_core::FuzzyRules;

/// 模糊音勾选框的 tag 起点，后面加规则在 [`FuzzyRules::NAMES`] 里的下标。
const FUZZY_TAG_BASE: NSInteger = 100;

/// 附加词库开关的 tag 起点，后面加词库在列表里的下标。
const DICTIONARY_ENABLED_TAG_BASE: NSInteger = 200;

/// 附加词库「移除」按钮的 tag 起点。
const DICTIONARY_REMOVE_TAG_BASE: NSInteger = 300;

/// 一页最多列多少本附加词库（tag 段的宽度）。
pub const MAX_DICTIONARIES: usize = 100;

/// 设置窗口里的每个控件对应的配置项。编码进控件的 tag，`changed:` 里再解出来。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    /// `[general] learning_language`，弹出菜单，选项是打进包里的释义表语言。
    LearningLanguage,

    /// `[general] page_size`，弹出菜单 1–9。
    PageSize,

    /// `[general] page_keys`，弹出菜单。
    PageKeys,

    /// `[general] theme`，弹出菜单。
    Theme,

    /// `[shortcut] expression`，弹出菜单 v / u / i。
    ExpressionKey,

    /// `[shortcut] question`，弹出菜单 v / u / i。
    QuestionKey,

    /// `[fuzzy]` 里的一条规则，值是 [`FuzzyRules::NAMES`] 的下标。
    Fuzzy(usize),

    /// `[predict] enabled`。
    CloudEnabled,

    /// `[model] enabled`。
    LocalModelEnabled,

    /// `[predict] base_url`。
    BaseUrl,

    /// `[predict] model`。
    Model,

    /// 密钥，写到 `.env`。
    ApiKey,

    /// 「在编辑器中打开配置文件」按钮。
    OpenConfigFile,

    /// `[general] layout`，弹出菜单 竖排 / 横排。
    Layout,

    /// `[general] preedit`，弹出菜单。
    Preedit,

    /// `[general] english_candidates`，勾选框。
    EnglishCandidates,

    /// `[shortcut] translation`，快捷键录制按钮（只记修饰键）。
    TranslationKeys,

    /// `[shortcut] translation_second`，同上。
    TranslationSecondKeys,

    /// `[shortcut] translate_selection`，快捷键录制按钮（修饰键 + 字母）。
    TranslateSelectionKeys,

    /// 「恢复默认快捷键」按钮：翻页键、模式键、三组译词 / 翻译快捷键全部回缺省。
    ResetShortcuts,

    /// 「导入词库…」按钮：选文件，转成 `.qj` 放进用户目录 `dicts/`。
    ImportDictionary,

    /// 第 N 本附加词库的启用勾选框（下标是 Host 词库列表里的位置）。
    DictionaryEnabled(usize),

    /// 第 N 本附加词库的「移除」按钮。
    DictionaryRemove(usize),

    /// `[general] shuangpin`，弹出菜单：关 + 四套方案。
    Shuangpin,

    /// `[general] log_level`，勾选框：勾上是 debug。
    VerboseLog,

    /// 「关于」页「打开日志目录」按钮。
    OpenLogDirectory,

    /// 「关于」页「复制诊断信息」按钮。
    CopyDiagnostics,

    /// `[predict] slots`，弹出菜单 0–4：第一页末尾留给云端词的格数。
    CloudSlots,

    /// `[apps] english_candidates_off`，勾选框：勾上写缺省的终端 / 编辑器列表，去掉写空表。
    EnglishCandidatesOffInApps,

    /// `[shortcut] delete_candidate`，快捷键录制按钮（只记修饰键）。
    DeleteCandidateKeys,

    /// `[general] input_log`，勾选框。
    InputLog,

    /// 「高级」页「清空输入日志」按钮。
    ClearInputLog,

    /// 「云服务」页「测试连接」按钮。
    TestCloud,

    /// 「关于」页「官网」按钮。
    OpenWebsite,

    /// 「关于」页「GitHub」按钮。
    OpenRepository,
}

impl Setting {
    pub fn tag(self) -> NSInteger {
        match self {
            Self::LearningLanguage => 1,
            Self::PageSize => 2,
            Self::PageKeys => 3,
            Self::Theme => 4,
            Self::ExpressionKey => 5,
            Self::QuestionKey => 6,
            Self::CloudEnabled => 7,
            Self::BaseUrl => 8,
            Self::Model => 9,
            Self::ApiKey => 10,
            Self::OpenConfigFile => 11,
            Self::Layout => 12,
            Self::Preedit => 13,
            Self::EnglishCandidates => 14,
            Self::TranslationKeys => 15,
            Self::TranslationSecondKeys => 16,
            Self::TranslateSelectionKeys => 17,
            Self::ResetShortcuts => 18,
            Self::ImportDictionary => 19,
            Self::Shuangpin => 20,
            Self::VerboseLog => 21,
            Self::OpenLogDirectory => 22,
            Self::CopyDiagnostics => 23,
            Self::CloudSlots => 24,
            Self::EnglishCandidatesOffInApps => 25,
            Self::DeleteCandidateKeys => 26,
            Self::InputLog => 27,
            Self::ClearInputLog => 28,
            Self::TestCloud => 29,
            Self::OpenWebsite => 30,
            Self::OpenRepository => 31,
            Self::LocalModelEnabled => 32,
            Self::Fuzzy(index) => FUZZY_TAG_BASE + index as NSInteger,
            Self::DictionaryEnabled(index) => DICTIONARY_ENABLED_TAG_BASE + index as NSInteger,
            Self::DictionaryRemove(index) => DICTIONARY_REMOVE_TAG_BASE + index as NSInteger,
        }
    }

    pub fn from_tag(tag: NSInteger) -> Option<Self> {
        Some(match tag {
            1 => Self::LearningLanguage,
            2 => Self::PageSize,
            3 => Self::PageKeys,
            4 => Self::Theme,
            5 => Self::ExpressionKey,
            6 => Self::QuestionKey,
            7 => Self::CloudEnabled,
            8 => Self::BaseUrl,
            9 => Self::Model,
            10 => Self::ApiKey,
            11 => Self::OpenConfigFile,
            12 => Self::Layout,
            13 => Self::Preedit,
            14 => Self::EnglishCandidates,
            15 => Self::TranslationKeys,
            16 => Self::TranslationSecondKeys,
            17 => Self::TranslateSelectionKeys,
            18 => Self::ResetShortcuts,
            19 => Self::ImportDictionary,
            20 => Self::Shuangpin,
            21 => Self::VerboseLog,
            22 => Self::OpenLogDirectory,
            23 => Self::CopyDiagnostics,
            24 => Self::CloudSlots,
            25 => Self::EnglishCandidatesOffInApps,
            26 => Self::DeleteCandidateKeys,
            27 => Self::InputLog,
            28 => Self::ClearInputLog,
            29 => Self::TestCloud,
            30 => Self::OpenWebsite,
            31 => Self::OpenRepository,
            32 => Self::LocalModelEnabled,
            _ if tag >= DICTIONARY_REMOVE_TAG_BASE => {
                let index = usize::try_from(tag - DICTIONARY_REMOVE_TAG_BASE).ok()?;
                (index < MAX_DICTIONARIES).then_some(Self::DictionaryRemove(index))?
            }
            _ if tag >= DICTIONARY_ENABLED_TAG_BASE => {
                let index = usize::try_from(tag - DICTIONARY_ENABLED_TAG_BASE).ok()?;
                (index < MAX_DICTIONARIES).then_some(Self::DictionaryEnabled(index))?
            }
            _ => {
                let index = usize::try_from(tag.checked_sub(FUZZY_TAG_BASE)?).ok()?;
                (index < FuzzyRules::NAMES.len()).then_some(Self::Fuzzy(index))?
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_round_trip() {
        let all = [
            Setting::LearningLanguage,
            Setting::PageSize,
            Setting::PageKeys,
            Setting::Theme,
            Setting::ExpressionKey,
            Setting::QuestionKey,
            Setting::CloudEnabled,
            Setting::LocalModelEnabled,
            Setting::BaseUrl,
            Setting::Model,
            Setting::ApiKey,
            Setting::OpenConfigFile,
            Setting::Layout,
            Setting::Preedit,
            Setting::EnglishCandidates,
            Setting::TranslationKeys,
            Setting::TranslationSecondKeys,
            Setting::TranslateSelectionKeys,
            Setting::ResetShortcuts,
            Setting::ImportDictionary,
            Setting::Shuangpin,
            Setting::VerboseLog,
            Setting::OpenLogDirectory,
            Setting::CopyDiagnostics,
            Setting::CloudSlots,
            Setting::EnglishCandidatesOffInApps,
            Setting::DeleteCandidateKeys,
            Setting::InputLog,
            Setting::ClearInputLog,
            Setting::TestCloud,
            Setting::OpenWebsite,
            Setting::OpenRepository,
            Setting::DictionaryEnabled(0),
            Setting::DictionaryEnabled(MAX_DICTIONARIES - 1),
            Setting::DictionaryRemove(3),
            Setting::Fuzzy(0),
            Setting::Fuzzy(FuzzyRules::NAMES.len() - 1),
        ];
        for setting in all {
            assert_eq!(Setting::from_tag(setting.tag()), Some(setting));
        }
        assert_eq!(Setting::from_tag(0), None);
        assert_eq!(
            Setting::from_tag(FUZZY_TAG_BASE + FuzzyRules::NAMES.len() as NSInteger),
            None
        );
        assert_eq!(
            Setting::from_tag(DICTIONARY_REMOVE_TAG_BASE + MAX_DICTIONARIES as NSInteger),
            None
        );
    }
}
