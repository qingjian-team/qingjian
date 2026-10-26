//! 符号面板的开关与搜索。面板开着时缓冲区里敲的字是搜索词，选中的条目上屏时吃掉整段（见 [`Engine::commit`]）。

use super::Engine;
use crate::candidate::{Candidate, CandidateKind};
use crate::emoji::builtin;
use crate::panel::{CATALOG, KEYWORDS, SymbolPanel};

/// 搜索拿前几个候选词去查关键词：再往后的词与敲的字关系就远了。
const SEARCH_WORDS: usize = 12;

/// 搜索结果最多几条，免得一屏屏翻不完。
const SEARCH_LIMIT: usize = 120;

impl Engine {
    /// 打开面板：在组句就拿当前候选搜一遍，搜到的放在最前面一组；没在组句（从菜单打开）就没有搜索组。
    /// 返回开没开（目前总是开）。
    pub fn open_symbol_panel(&mut self) -> bool {
        if self.composition.is_empty() {
            self.panel = Some(SymbolPanel::new("", Vec::new(), self.panel_history.uses()));
            return true;
        }
        let items = self
            .query()
            .map(|query| query.candidates.items)
            .unwrap_or_default();
        let results = self.search_panel(&items);
        self.panel = Some(SymbolPanel::new(
            self.composition.text(),
            results,
            self.panel_history.uses(),
        ));
        true
    }

    pub fn symbol_panel(&self) -> Option<&SymbolPanel> {
        self.panel.as_ref()
    }

    pub fn symbol_panel_mut(&mut self) -> Option<&mut SymbolPanel> {
        self.panel.as_mut()
    }

    pub fn close_symbol_panel(&mut self) {
        self.panel = None;
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
                .iter()
                .chain(self.emoji.as_ref().map_or(&[][..], |t| t.lookup(&key)));
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
                        text: text.clone(),
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
