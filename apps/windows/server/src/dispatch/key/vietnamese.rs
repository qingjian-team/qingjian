//! Windows 越南语模式：Server 持有 Simple Telex，TSF 只显示返回的无候选 preedit。

use qingjian_core::Translator;
use qingjian_core::{Candidate, CandidateKind};
use qingjian_platform::protocol::KeyEvent;

use super::{Effect, codes};
use crate::dispatch::{Composed, Router};

impl Router {
    pub(super) fn apply_vietnamese(&mut self, event: &KeyEvent) -> Effect {
        if event.modifiers.has_command_key() {
            return self.finish_vietnamese(false);
        }
        match event.virtual_key {
            codes::BACK => {
                if self.vietnamese.backspace() {
                    self.sync_vietnamese_preedit();
                    return Effect::Changed(None);
                }
                return Effect::Passthrough;
            }
            codes::ESCAPE => {
                if !self.vietnamese.restore() {
                    self.vietnamese.clear();
                }
                self.sync_vietnamese_preedit();
                return Effect::Changed(None);
            }
            codes::RETURN => return self.finish_vietnamese(true),
            codes::TAB => {
                return self
                    .commit_index(self.highlight)
                    .map_or(Effect::Navigated, |text| Effect::Changed(Some(text)));
            }
            codes::DOWN | codes::RIGHT => {
                self.move_highlight(1);
                return Effect::Navigated;
            }
            codes::UP | codes::LEFT => {
                self.move_highlight(-1);
                return Effect::Navigated;
            }
            _ => {}
        }
        let Some(c) = event.character.filter(|c| !c.is_control()) else {
            return if self.vietnamese.is_empty() {
                Effect::Passthrough
            } else {
                self.finish_vietnamese(true)
            };
        };
        if let Some(digit) = c.to_digit(10)
            && let Some(index) = self.slot_index(digit as usize)
            && let Some(text) = self.commit_index(index)
        {
            return Effect::Changed(Some(text));
        }
        if c.is_ascii_alphabetic() {
            self.vietnamese.push(c);
            self.sync_vietnamese_preedit();
            return Effect::Changed(None);
        }
        if self.vietnamese.is_empty() {
            return Effect::Passthrough;
        }
        if self.vietnamese.commit_word(None, c) {
            self.sync_vietnamese_preedit();
            Effect::Changed(None)
        } else {
            Effect::Passthrough
        }
    }

    fn finish_vietnamese(&mut self, consume_boundary: bool) -> Effect {
        if self.vietnamese.is_empty() {
            return if consume_boundary {
                Effect::Navigated
            } else {
                Effect::Passthrough
            };
        }
        let text = self.vietnamese.commit();
        self.composed = None;
        Effect::Changed(Some(text))
    }

    pub(crate) fn sync_vietnamese_preedit(&mut self) {
        let text = self.vietnamese.preedit().to_owned();
        self.composed = if text.is_empty() {
            None
        } else {
            let raw = self.vietnamese.raw();
            let current = self.vietnamese.current();
            let phrase_prefix = self.vietnamese.phrase_prefix();
            let mut candidates: Vec<Candidate> = Vec::new();
            if let Some(words) = &self.vietnamese_words {
                for word in qingjian_core::suggest_vietnamese(words, phrase_prefix, current, 7) {
                    if !candidates.iter().any(|candidate| candidate.text == word) {
                        let translation = self
                            .vietnamese_glossary
                            .as_ref()
                            .and_then(|glossary| glossary.translate(&word));
                        candidates.push(vietnamese_candidate(&word, translation));
                    }
                }
            }
            if !current.is_empty() && !candidates.iter().any(|candidate| candidate.text == current)
            {
                let translation = self
                    .vietnamese_glossary
                    .as_ref()
                    .and_then(|glossary| glossary.translate(current));
                let phrase_count = candidates
                    .iter()
                    .take_while(|candidate| candidate.text.starts_with(phrase_prefix))
                    .count();
                candidates.insert(phrase_count, vietnamese_candidate(current, translation));
            }
            if !current.is_empty() && raw != current {
                candidates.push(vietnamese_candidate(raw, None));
            }
            self.engine
                .rank_external_candidates(&format!("vi:{raw}"), &mut candidates);
            Some(Composed::Vietnamese { text, candidates })
        };
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
