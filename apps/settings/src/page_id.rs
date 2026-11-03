//! 侧栏的各页，顺序与名字见 docs/design/settings.md。

use crate::i18n::tr;
use crate::support::Feature;
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
    Experimental,
    About,
}

impl PageId {
    pub const ALL: [Self; 13] = [
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
        Self::Experimental,
        Self::About,
    ];

    /// 这个平台的侧栏有这一页（实验功能只在做了的平台出现）。
    pub fn shown(self) -> bool {
        self != Self::Experimental
            || Feature::ExperimentalModels.here()
            || Feature::Clipboard.here()
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Input => tr("page.input"),
            Self::Candidates => tr("page.candidates"),
            Self::Language => tr("page.language"),
            Self::Appearance => tr("page.appearance"),
            Self::Shortcuts => tr("page.shortcuts"),
            Self::Phrases => tr("page.phrases"),
            Self::Dictionaries => tr("page.dictionaries"),
            Self::Cloud => tr("page.cloud"),
            Self::Stats => tr("page.stats"),
            Self::Vocabulary => tr("page.vocabulary"),
            Self::Advanced => tr("page.advanced"),
            Self::Experimental => tr("page.experimental"),
            Self::About => tr("page.about"),
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
            Self::Experimental => Glyph::Experimental,
            Self::About => Glyph::About,
        }
    }
}
