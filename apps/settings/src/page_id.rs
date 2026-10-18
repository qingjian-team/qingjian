//! 侧栏的各页，顺序与名字见 docs/design/settings.md。

use crate::view::icons::Glyph;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageId {
    Input,
    Candidates,
    Language,
    Appearance,
    Shortcuts,
    Phrases,
    Dictionaries,
    Cloud,
    Stats,
    Vocabulary,
    Advanced,
    About,
}

impl PageId {
    pub const ALL: [Self; 12] = [
        Self::Input,
        Self::Candidates,
        Self::Language,
        Self::Appearance,
        Self::Shortcuts,
        Self::Phrases,
        Self::Dictionaries,
        Self::Cloud,
        Self::Stats,
        Self::Vocabulary,
        Self::Advanced,
        Self::About,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Input => "输入",
            Self::Candidates => "候选",
            Self::Language => "中英文",
            Self::Appearance => "外观",
            Self::Shortcuts => "快捷键",
            Self::Phrases => "自定义短语",
            Self::Dictionaries => "词库",
            Self::Cloud => "云服务",
            Self::Stats => "统计",
            Self::Vocabulary => "生词本",
            Self::Advanced => "高级",
            Self::About => "关于",
        }
    }

    pub fn glyph(self) -> Glyph {
        match self {
            Self::Input => Glyph::Keyboard,
            Self::Candidates => Glyph::Candidates,
            Self::Language => Glyph::Language,
            Self::Appearance => Glyph::Appearance,
            Self::Shortcuts => Glyph::Command,
            Self::Phrases => Glyph::Phrases,
            Self::Dictionaries => Glyph::Dictionary,
            Self::Cloud => Glyph::Cloud,
            Self::Stats => Glyph::Stats,
            Self::Vocabulary => Glyph::Vocabulary,
            Self::Advanced => Glyph::Advanced,
            Self::About => Glyph::About,
        }
    }
}
