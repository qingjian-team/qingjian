//! 只在部分平台显示的设置项：各平台的输入法读不读它，设置程序就按这张表显示或隐藏。
//! 新加的设置项若只做了一个平台，在这里登记另外两个平台是「不适用」还是「待实现」。

mod platform;
mod status;

pub use platform::Platform;
pub use status::Status;

#[cfg(test)]
use crate::page_id::PageId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    /// 输入页的辅码与码表。
    AuxCode,
    /// 青简内部的中英文切换键。
    SwitchKeys,
    /// Windows 系统的 Ctrl + Space，与青简内部切换键独立。
    SystemCtrlSpace,
    DefaultEnglish,
    RememberModePerApp,
    /// 英文模式的全半角标点。
    EnglishPunctuation,
    AutoSpace,
    StatusBar,
    /// 横排时 ↑ / ↓ 展开成多行。
    HorizontalGrid,
    ExtraPageKeys,
    HighlightKeys,
    TogglePunctuation,
    SystemTextReplacements,
    /// 统计页的云服务用量，测试连接也记进去。
    ApiUsage,
    /// Windows 的手动云联想与请求节奏设置。
    CloudRequestControls,
    /// 上屏的整句翻译后记进生词本（随释义兜底）。
    VocabularyTranslation,
    /// 本机 Ollama 实时中英互译。
    LiveTranslation,
}

impl Feature {
    #[cfg(test)]
    pub const ALL: [Self; 17] = [
        Self::AuxCode,
        Self::SwitchKeys,
        Self::SystemCtrlSpace,
        Self::DefaultEnglish,
        Self::RememberModePerApp,
        Self::EnglishPunctuation,
        Self::AutoSpace,
        Self::StatusBar,
        Self::HorizontalGrid,
        Self::ExtraPageKeys,
        Self::HighlightKeys,
        Self::TogglePunctuation,
        Self::SystemTextReplacements,
        Self::ApiUsage,
        Self::CloudRequestControls,
        Self::VocabularyTranslation,
        Self::LiveTranslation,
    ];

    /// 当前平台显示这一项。
    pub fn here(self) -> bool {
        self.status(Platform::CURRENT) == Status::Supported
    }

    pub fn status(self, platform: Platform) -> Status {
        let [macos, windows, linux] = self.table();
        match platform {
            Platform::MacOs => macos,
            Platform::Windows => windows,
            Platform::Linux => linux,
        }
    }

    /// macOS、Windows、Linux 依次。
    fn table(self) -> [Status; 3] {
        use Status::{NotApplicable as No, Planned as Todo, Supported as Yes};
        match self {
            Self::AuxCode => [Todo, Yes, Todo],
            // Linux 的切换键在 Fcitx5 的设置里
            Self::SwitchKeys => [Yes, Yes, No],
            Self::SystemCtrlSpace => [No, Yes, No],
            Self::DefaultEnglish => [Todo, Yes, Todo],
            Self::RememberModePerApp => [Todo, Yes, Todo],
            // macOS 英文模式一律半角
            Self::EnglishPunctuation => [No, Yes, Yes],
            // 要读应用里光标前的字
            Self::AutoSpace => [Yes, Todo, Todo],
            // macOS 看菜单栏图标，Linux 看 Fcitx5 的托盘
            Self::StatusBar => [No, Yes, No],
            Self::HorizontalGrid => [Yes, Yes, Todo],
            Self::ExtraPageKeys => [Todo, Yes, Todo],
            Self::HighlightKeys => [Yes, Yes, Todo],
            Self::TogglePunctuation => [Todo, Yes, Todo],
            // 系统的文本替换只有 macOS 有
            Self::SystemTextReplacements => [Yes, No, No],
            // Linux 已接云服务，尚未记录云服务用量
            Self::ApiUsage => [Yes, Yes, Todo],
            Self::CloudRequestControls => [Todo, Yes, Todo],
            // Linux 还没接释义兜底
            Self::VocabularyTranslation => [Yes, Yes, Todo],
            Self::LiveTranslation => [Yes, Todo, Todo],
        }
    }

    /// 在 `docs/plan/todo.md` 里的叫法。
    #[cfg(test)]
    pub fn label(self) -> &'static str {
        match self {
            Self::AuxCode => "辅码",
            Self::SwitchKeys => "中英文切换键",
            Self::SystemCtrlSpace => "系统 Ctrl + Space",
            Self::DefaultEnglish => "默认英文模式",
            Self::RememberModePerApp => "按应用记住中英模式",
            Self::EnglishPunctuation => "英文标点全半角",
            Self::AutoSpace => "中文与英文、数字之间自动加空格",
            Self::StatusBar => "悬浮状态条",
            Self::HorizontalGrid => "横排时 ↑ / ↓ 展开成多行",
            Self::ExtraPageKeys => "额外翻页键",
            Self::HighlightKeys => "高亮移动键",
            Self::TogglePunctuation => "Ctrl + . 切换全半角标点",
            Self::SystemTextReplacements => "也用系统的文本替换",
            Self::ApiUsage => "云服务用量统计",
            Self::CloudRequestControls => "按需云联想与请求节奏",
            Self::VocabularyTranslation => "生词本整句翻译",
            Self::LiveTranslation => "本地实时中英互译",
        }
    }

    /// 这一项建出来的、带 ⓘ 链接的行（页、行名）；不显示时这些行也没有。
    #[cfg(test)]
    pub fn rows(self) -> &'static [(PageId, &'static str)] {
        match self {
            Self::AuxCode => &[
                (PageId::Input, "启用"),
                (PageId::Input, "触发键"),
                (PageId::Input, "导入码表"),
            ],
            Self::SwitchKeys => &[(PageId::Language, "切换键")],
            Self::DefaultEnglish => &[(PageId::Language, "默认英文模式")],
            Self::RememberModePerApp => &[(PageId::Language, "按应用记住中英模式")],
            Self::AutoSpace => &[(PageId::Language, "中文与英文、数字之间自动加空格")],
            Self::ExtraPageKeys => &[(PageId::Shortcuts, "额外翻页键")],
            Self::HighlightKeys => &[(PageId::Shortcuts, "高亮移动键")],
            Self::TogglePunctuation => &[(PageId::Shortcuts, "Ctrl + . 切换全半角标点")],
            Self::SystemTextReplacements => &[(PageId::Phrases, "也用系统的文本替换")],
            Self::CloudRequestControls => &[
                (PageId::Cloud, "自动联想"),
                (PageId::Cloud, "停键等待"),
                (PageId::Cloud, "最短请求间隔"),
                (PageId::Shortcuts, "按需 AI 联想"),
            ],
            Self::VocabularyTranslation => &[(PageId::Cloud, "生词本整句翻译")],
            Self::LiveTranslation => &[
                (PageId::Language, "实时中英互译"),
                (PageId::Language, "本机服务地址"),
                (PageId::Language, "本地翻译模型"),
            ],
            Self::SystemCtrlSpace
            | Self::EnglishPunctuation
            | Self::StatusBar
            | Self::HorizontalGrid
            | Self::ApiUsage => &[],
        }
    }
}

/// 某页的某行只在别的平台建出来。
#[cfg(test)]
pub fn elsewhere(page: PageId, row: &str) -> bool {
    Feature::ALL
        .iter()
        .any(|feature| !feature.here() && feature.rows().contains(&(page, row)))
}

#[cfg(test)]
mod tests {
    use super::{Feature, Platform, Status};

    /// `docs/plan/todo.md` 里「设置项的平台缺口」下的条目与表里的「待实现」一一对应。
    #[test]
    fn planned_gaps_are_listed_in_todo() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/plan/todo.md");
        let todo = std::fs::read_to_string(path).unwrap();
        let listed: Vec<&str> = todo
            .lines()
            .skip_while(|line| !line.contains("设置项的平台缺口"))
            .skip(1)
            .take_while(|line| line.starts_with("  "))
            .collect();
        let expected: Vec<String> = Feature::ALL
            .iter()
            .filter_map(|feature| {
                let platforms: Vec<&str> = Platform::ALL
                    .iter()
                    .filter(|platform| feature.status(**platform) == Status::Planned)
                    .map(|platform| platform.label())
                    .collect();
                (!platforms.is_empty())
                    .then(|| format!("  - [ ] {}：{}", feature.label(), platforms.join("、")))
            })
            .collect();
        assert_eq!(
            listed, expected,
            "todo.md 的「设置项的平台缺口」要照 support 表的「待实现」逐条列"
        );
    }

    #[test]
    fn internal_switch_keys_and_system_hotkey_have_separate_support() {
        assert_eq!(
            Feature::SwitchKeys.status(Platform::MacOs),
            Status::Supported
        );
        assert_eq!(
            Feature::SwitchKeys.status(Platform::Windows),
            Status::Supported
        );
        assert_eq!(
            Feature::SwitchKeys.status(Platform::Linux),
            Status::NotApplicable
        );
        for platform in Platform::ALL {
            assert_eq!(
                Feature::SystemCtrlSpace.status(platform),
                if platform == Platform::Windows {
                    Status::Supported
                } else {
                    Status::NotApplicable
                }
            );
        }
    }

    /// 每一项至少一个平台在用，不然该删掉。
    #[test]
    fn every_feature_is_supported_somewhere() {
        for feature in Feature::ALL {
            assert!(
                Platform::ALL
                    .iter()
                    .any(|platform| feature.status(*platform) == Status::Supported),
                "{} 哪个平台都没有",
                feature.label()
            );
        }
    }
}
