//! 符号面板的开关与搜索。面板开着时缓冲区里敲的字是搜索词，选中的条目上屏时吃掉整段（见 [`Engine::commit`]）；
//! 翻译组的原文与翻译在 [`translate`]。

mod translate;

use super::Engine;
use crate::candidate::{Candidate, CandidateKind};
use crate::emoji::builtin;
use crate::panel::{CATALOG, KEYWORDS, PanelGroup, SymbolPanel};

/// 搜索拿前几个候选词去查关键词：再往后的词与敲的字关系就远了。
const SEARCH_WORDS: usize = 12;

/// 搜索结果最多几条，免得一屏屏翻不完。
const SEARCH_LIMIT: usize = 120;

impl Engine {
    /// 打开面板：在组句就拿当前候选搜一遍，搜到的放进搜索组，停在翻译组（带着这段拼音接着打）；
    /// 没在组句（从菜单打开）就没有搜索组，停在上次关掉时那组。翻译不可用时没有翻译组。
    /// 返回开没开（目前总是开）。
    pub fn open_symbol_panel(&mut self) -> bool {
        let composing = !self.composition.is_empty();
        let mut panel = if composing {
            let items = self
                .query()
                .map(|query| query.candidates.items)
                .unwrap_or_default();
            let results = self.search_panel(&items);
            SymbolPanel::new(self.composition.text(), results, self.panel_history.uses())
        } else {
            SymbolPanel::new("", Vec::new(), self.panel_history.uses())
        };
        if self.panel_translate_available() {
            panel = panel.with_translate(self.panel_language());
        }
        let landing = if composing {
            Some(PanelGroup::Translate)
        } else {
            self.panel_group
        };
        if let Some(group) = landing {
            panel.open_on(group);
        }
        self.panel = Some(panel);
        true
    }

    pub fn symbol_panel(&self) -> Option<&SymbolPanel> {
        self.panel.as_ref()
    }

    pub fn symbol_panel_mut(&mut self) -> Option<&mut SymbolPanel> {
        self.panel.as_mut()
    }

    pub fn close_symbol_panel(&mut self) {
        self.take_panel();
    }

    /// 收起面板：记下停在哪组、翻译选的语言，作废翻译框在飞的请求（免得译文回来被当成别的结果）。
    pub(in crate::engine) fn take_panel(&mut self) -> Option<SymbolPanel> {
        let panel = self.panel.take()?;
        self.panel_group = Some(panel.group());
        if let Some(language) = panel.target_language() {
            self.panel_language = Some(language);
        }
        if let Some(sequence) = panel.draft().in_flight() {
            self.pending_predictions.remove(&sequence);
        }
        Some(panel)
    }

    /// 候选里已经配好的 emoji / 颜文字 / 符号先收，再拿前几个词查关键词与三张内置表。
    /// 关键词与 emoji 表查出的只收面板里有的（版本太新、系统显示不出来的不出）；内置符号与颜文字表的照收。
    fn search_panel(&self, items: &[Candidate]) -> Vec<Candidate> {
        let mut found: Vec<Candidate> = Vec::new();
        let mut push = |candidate: Candidate| {
            if found.len() < SEARCH_LIMIT && !found.iter().any(|c| c.text == candidate.text) {
                found.push(candidate);
            }
        };
        for item in items.iter().filter(|c| c.kind.follows_word()) {
            push(
                CATALOG
                    .item(&item.text)
                    .cloned()
                    .unwrap_or_else(|| Candidate {
                        syllables: Vec::new(),
                        ..item.clone()
                    }),
            );
        }
        let words = items
            .iter()
            .filter(|c| {
                matches!(
                    c.kind,
                    CandidateKind::Chinese | CandidateKind::Sentence | CandidateKind::English
                )
            })
            .take(SEARCH_WORDS);
        for word in words {
            let key = if word.kind == CandidateKind::English {
                word.text.to_lowercase()
            } else {
                word.text.clone()
            };
            let catalogued = KEYWORDS
                .lookup(&key)
                .chain(self.emoji.iter().flat_map(|t| t.lookup(&key)));
            for text in catalogued {
                if let Some(item) = CATALOG.item(text) {
                    push(item.clone());
                }
            }
            for (table, kind) in [
                (&*builtin::SYMBOLS, CandidateKind::Symbol),
                (&*builtin::KAOMOJI, CandidateKind::Kaomoji),
            ] {
                for text in table.lookup(&key) {
                    push(CATALOG.item(text).cloned().unwrap_or_else(|| Candidate {
                        text: text.to_owned(),
                        kind,
                        syllables: Vec::new(),
                        reading: Some(word.text.clone()),
                        translation: None,
                        aux_code: None,
                    }));
                }
            }
        }
        found
    }
}
