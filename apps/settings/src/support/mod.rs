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
    /// 形码，以及用户目录 `schemes/` 里的形码与双拼方案。
    Schemes,
    /// 中英文切换键。
    SwitchKeys,
    /// 切换键里的「Ctrl + Space（系统）」：改的是 Windows 的系统热键。
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
    /// 双拼键位提示：候选窗旁画键盘。
    KeyHint,
    /// 拼音行按高亮候选显示带调拼音。
    ShowTones,
    /// 「实验功能」页：下载、启用知意与含章·知微。
    ExperimentalModels,
}

impl Feature {
    #[cfg(test)]
    pub const ALL: [Self; 20] = [
        Self::AuxCode,
        Self::Schemes,
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
        Self::KeyHint,
        Self::ShowTones,
        Self::ExperimentalModels,
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
            Self::AuxCode => [Yes, Yes, Todo],
            Self::Schemes => [Yes, Yes, Todo],
            // macOS 用 Caps Lock，由系统决定
            Self::SwitchKeys => [No, Yes, Yes],
            Self::SystemCtrlSpace => [No, Yes, No],
            Self::DefaultEnglish => [Todo, Yes, Yes],
            Self::RememberModePerApp => [Todo, Yes, Todo],
            // macOS 英文模式一律半角
            Self::EnglishPunctuation => [No, Yes, Yes],
            // 要读应用里光标前的字
            Self::AutoSpace => [Yes, Yes, Todo],
            // macOS 看菜单栏图标，Linux 看 Fcitx5 的托盘
            Self::StatusBar => [No, Yes, No],
            Self::HorizontalGrid => [Yes, Yes, Todo],
            Self::ExtraPageKeys => [Yes, Yes, Todo],
            Self::HighlightKeys => [Yes, Yes, Todo],
            Self::TogglePunctuation => [Yes, Yes, Todo],
            // 系统的文本替换只有 macOS 有
            Self::SystemTextReplacements => [Yes, No, No],
            Self::ApiUsage => [Yes, Yes, Yes],
            Self::CloudRequestControls => [Yes, Yes, Yes],
            Self::VocabularyTranslation => [Yes, Yes, Yes],
            Self::KeyHint => [Yes, Yes, Todo],
            // Fcitx 面板与应用里的行内拼音共用一串，还没分开
            Self::ShowTones => [Yes, Yes, Todo],
            // Windows、Linux 的服务进程还没接知意；没有 GPU 时 CPU 上打分要发烫，先只开 macOS（Metal）
            Self::ExperimentalModels => [Yes, Yes, Todo],
        }
    }

    /// 在 `docs/plan/todo.md` 里的叫法。
    #[cfg(test)]
    pub fn label(self) -> &'static str {
        match self {
            Self::AuxCode => "辅码",
            Self::Schemes => "形码与自定义方案",
            Self::SwitchKeys => "中英文切换键",
            Self::SystemCtrlSpace => "系统的 Ctrl + Space",
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
            Self::KeyHint => "双拼键位提示",
            Self::ShowTones => "拼音行显示声调",
            Self::ExperimentalModels => "实验功能（知意、含章·知微）",
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
            Self::Schemes => &[(PageId::Input, "形码")],
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
            Self::KeyHint => &[(PageId::Input, "键位提示")],
            Self::ShowTones => &[(PageId::Candidates, "拼音行显示声调")],
            Self::SystemCtrlSpace
            | Self::EnglishPunctuation
            | Self::StatusBar
            | Self::HorizontalGrid
            | Self::ApiUsage
            | Self::ExperimentalModels => &[],
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
