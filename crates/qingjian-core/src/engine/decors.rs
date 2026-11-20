//! Engine 的符号装饰：emoji / 颜文字 / 符号表与符号面板。

use crate::candidate::Language;
use crate::emoji::EmojiTable;
use crate::panel::{NoPanelHistory, PanelGroup, PanelHistory, SymbolPanel, TranslationCache};

/// 符号相关的全部状态：三张表、打开着的面板、面板的使用记录与翻译缓存。
pub(crate) struct EngineDecors {
    /// emoji 表，没有就不出 emoji 候选。
    pub(crate) emoji: Option<EmojiTable>,

    /// 内置的颜文字表，带上 emoji 表时一起启用。
    pub(crate) kaomoji: Option<&'static EmojiTable>,

    /// 内置的符号表，带上 emoji 表时一起启用。
    pub(crate) symbols: Option<&'static EmojiTable>,

    /// 打开着的符号面板；`None` 是没开。上屏与清空时关掉。
    pub(crate) panel: Option<SymbolPanel>,

    /// 面板条目的使用记录：打开面板时排「常用」，从面板上屏时记一次；缺省不记。
    pub(crate) panel_history: Box<dyn PanelHistory>,

    /// 面板上次关掉时停在哪组：没在组句时打开就回到那组。
    pub(crate) panel_group: Option<PanelGroup>,

    /// 翻译组上次选的目标语言；没选过跟学习语言走。
    pub(crate) panel_language: Option<Language>,

    /// 翻译组的译文缓存，换联想提供方时清空。
    pub(crate) panel_translations: TranslationCache,
}

impl Default for EngineDecors {
    fn default() -> Self {
        Self {
            emoji: None,
            kaomoji: None,
            symbols: None,
            panel: None,
            panel_history: Box::new(NoPanelHistory),
            panel_group: None,
            panel_language: None,
            panel_translations: TranslationCache::default(),
        }
    }
}
