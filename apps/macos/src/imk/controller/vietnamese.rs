//! macOS 越南语模式：Simple Telex 放在 marked text，词边界一次性提交。

use objc2_app_kit::{NSEvent, NSEventModifierFlags};
use qingjian_core::Translator;
use qingjian_core::{Candidate, CandidateKind};

use super::QingjianInputController;
use crate::host;
use crate::imk::TextClient;

impl QingjianInputController {
    pub(super) fn handle_vietnamese_event(&self, event: &NSEvent, client: TextClient<'_>) -> bool {
        let flags = event.modifierFlags();
        if flags.intersects(NSEventModifierFlags::Command | NSEventModifierFlags::Control) {
            self.commit_vietnamese(client);
            return false;
        }
        match event.keyCode() {
            51 => {
                if host::with(|h| h.vietnamese.backspace()).unwrap_or(false) {
                    self.show_vietnamese(client);
                    return true;
                }
                return false;
            }
            53 => {
                host::with(|h| {
                    if !h.vietnamese.restore() {
                        h.vietnamese.clear();
                    }
                });
                self.show_vietnamese(client);
                return true;
            }
            36 | 76 => {
                self.commit_vietnamese(client);
                return true;
            }
            48 => {
                let index = host::with(|h| h.session.highlighted).unwrap_or(0);
                return self.choose_vietnamese_candidate(index, ' ', client);
            }
            125 => {
                self.move_highlight(1, client);
                return true;
            }
            126 => {
                self.move_highlight(-1, client);
                return true;
            }
            123 | 124 => {
                // 越南语整段暂不支持行内光标编辑；候选存在时左右键也按候选移动。
                let delta = if event.keyCode() == 123 { -1 } else { 1 };
                self.move_highlight(delta, client);
                return true;
            }
            _ => {}
        }
        let Some(text) = event.characters().map(|text| text.to_string()) else {
            return false;
        };
        let mut chars = text.chars();
        let Some(c) = chars.next() else {
            return false;
        };
        if chars.next().is_some() {
            self.commit_vietnamese(client);
            return false;
        }
        if c.is_ascii_alphabetic() {
            host::with(|h| {
                h.vietnamese.push(c);
            });
            self.show_vietnamese(client);
            return true;
        }
        if let Some(offset) = c.to_digit(10).and_then(|digit| digit.checked_sub(1))
            && let Some(index) = host::with(|h| h.session.index_on_page(offset as usize)).flatten()
            && self.choose_vietnamese_candidate(index, ' ', client)
        {
            return true;
        }
        let committed = host::with(|h| h.vietnamese.commit_word(None, c)).unwrap_or(false);
        if !committed {
            return false;
        }
        self.show_vietnamese(client);
        true
    }

    pub(super) fn commit_vietnamese(&self, client: TextClient<'_>) {
        let text = host::with(|h| {
            let text = h.vietnamese.commit();
            h.reset_session(None, Vec::new());
            h.window.hide();
            text
        })
        .unwrap_or_default();
        if !text.is_empty() {
            client.set_marked_text("", 0);
            client.insert_text(&text);
        }
    }

    pub(super) fn show_vietnamese(&self, client: TextClient<'_>) {
        let text = host::with(|h| {
            let text = h.vietnamese.preedit().to_owned();
            let current = h.vietnamese.current().to_owned();
            let phrase_prefix = h.vietnamese.phrase_prefix().to_owned();
            let raw = h.vietnamese.raw().to_owned();
            let mut candidates: Vec<Candidate> = Vec::new();
            if !current.is_empty()
                && let Some(words) = &h.vietnamese_words
            {
                for word in qingjian_core::suggest_vietnamese(words, &phrase_prefix, &current, 7) {
                    if !candidates.iter().any(|candidate| candidate.text == word) {
                        let translation = h
                            .vietnamese_glossary
                            .as_ref()
                            .and_then(|glossary| glossary.translate(&word));
                        candidates.push(vietnamese_candidate(&word, translation));
                    }
                }
            }
            if !current.is_empty() && !candidates.iter().any(|candidate| candidate.text == current)
            {
                let translation = h
                    .vietnamese_glossary
                    .as_ref()
                    .and_then(|glossary| glossary.translate(&current));
                // 连续短语优先；当前词本身放在短语之后、当前词补全之前较难从平铺结果区分，
                // 至少保证不再固定压过完整短语。
                let phrase_count = candidates
                    .iter()
                    .take_while(|candidate| candidate.text.starts_with(&phrase_prefix))
                    .count();
                candidates.insert(phrase_count, vietnamese_candidate(&current, translation));
            }
            if raw != current {
                candidates.push(vietnamese_candidate(&raw, None));
            }
            h.engine
                .rank_external_candidates(&format!("vi:{raw}"), &mut candidates);
            let preedit = if text.is_empty() {
                None
            } else {
                crate::candidates::Preedit::plain(&text, text.chars().count())
            };
            h.reset_session(
                preedit,
                if current.is_empty() {
                    Vec::new()
                } else {
                    candidates
                },
            );
            text
        })
        .unwrap_or_default();
        let cursor = text.chars().count();
        // 与中文 refresh 顺序一致：先把 marked text 交给应用，再查询光标位置、显示候选窗。
        // 反过来会在部分 IMK 客户端首键时造成重入，Objective-C exception 越过 Rust FFI 后 abort。
        if text.is_empty() {
            client.set_marked_text("", 0);
            host::with(|h| h.window.hide());
        } else {
            client.set_marked_text(&text, cursor);
            let has_candidates = host::with(|h| !h.session.layout.is_empty()).unwrap_or(false);
            if has_candidates {
                let anchor = client.caret_rect();
                host::with(|h| {
                    h.anchor = anchor;
                    h.render();
                });
            } else {
                host::with(|h| h.window.hide());
            }
        }
    }

    pub(super) fn commit_vietnamese_candidate(&self, index: usize, client: TextClient<'_>) -> bool {
        self.choose_vietnamese_candidate(index, ' ', client)
    }

    fn choose_vietnamese_candidate(
        &self,
        index: usize,
        boundary: char,
        client: TextClient<'_>,
    ) -> bool {
        let selected = host::with(|h| {
            let candidate = h.session.candidate(index)?.text;
            let phrase_prefix = h.vietnamese.phrase_prefix();
            let phrase =
                phrase_prefix != h.vietnamese.current() && candidate.starts_with(phrase_prefix);
            if phrase {
                h.vietnamese.choose_phrase(&candidate, boundary)
            } else {
                h.vietnamese.commit_word(Some(&candidate), boundary)
            }
            .then_some(())
        })
        .flatten()
        .is_some();
        if selected {
            self.show_vietnamese(client);
        }
        selected
    }
}

fn vietnamese_candidate(text: &str, translation: Option<qingjian_core::Translation>) -> Candidate {
    Candidate {
        text: text.to_owned(),
        kind: CandidateKind::English,
        syllables: Vec::new(),
        reading: None,
        translation,
        aux_code: None,
    }
}
