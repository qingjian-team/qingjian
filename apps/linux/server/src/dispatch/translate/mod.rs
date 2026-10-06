//! Linux 选区翻译：请求应用选区、等待云端译文并生成评审候选。
mod state;

use super::Router;
use qingjian_core::{Candidate, CandidateKind, CandidateList};
use qingjian_platform::protocol::{
    Frame, KeyEvent, KeyModifiers, KeyOutcome, ServerMessage, SessionId,
};

pub(super) use state::Translation;

impl Router {
    pub(super) fn matches_translate_combo(&self, event: &KeyEvent) -> bool {
        let combo = self.config.translate_selection;
        event
            .character
            .is_some_and(|c| c.eq_ignore_ascii_case(&combo.key))
            && event.modifiers.chord() == KeyModifiers::from(combo.modifiers)
    }

    pub(super) fn request_selection(&mut self, session: SessionId) -> ServerMessage {
        self.selection_seq += 1;
        self.pending_selection = Some((session, self.selection_seq));
        ServerMessage::RequestSelection {
            session,
            request: self.selection_seq,
        }
    }

    pub(super) fn receive_selection(
        &mut self,
        session: SessionId,
        request: u64,
        text: String,
    ) -> KeyOutcome {
        if self.pending_selection != Some((session, request))
            || self.focused != Some(session)
            || self.sessions[&session].private
            || !self.sessions[&session].active
            || !self.engine.composition().is_empty()
            || text.chars().count() > 500
            || text.trim().is_empty()
        {
            return KeyOutcome::Passthrough;
        }
        self.pending_selection = None;
        if self.engine.request_translation(text.trim()).is_none() {
            return KeyOutcome::Passthrough;
        }
        self.translation = Some(Translation { result: None });
        KeyOutcome::Consumed
    }

    pub(super) fn review_translation(&mut self, event: &KeyEvent) -> (KeyOutcome, Option<String>) {
        let accept = event.virtual_key == 0x0d || matches!(event.character, Some(' ') | Some('1'));
        if accept {
            let result = self.translation.as_ref().and_then(|job| job.result.clone());
            if result.is_some() {
                self.end_translation();
            }
            return (KeyOutcome::Consumed, result);
        }
        let escape = event.virtual_key == 0x1b;
        self.end_translation();
        (
            if escape {
                KeyOutcome::Consumed
            } else {
                KeyOutcome::Passthrough
            },
            None,
        )
    }

    pub(super) fn end_translation(&mut self) {
        self.pending_selection = None;
        if self.translation.take().is_some() {
            self.engine.cancel_prediction();
        }
    }

    pub(super) fn translation_frame(&self) -> Frame {
        let text = self
            .translation
            .as_ref()
            .and_then(|job| job.result.clone())
            .unwrap_or_else(|| "翻译中…".to_owned());
        Frame {
            candidates: CandidateList {
                items: vec![Candidate {
                    text,
                    kind: CandidateKind::Cloud,
                    syllables: Vec::new(),
                    reading: None,
                    translation: None,
                    aux_code: None,
                }],
            },
            highlight: 0,
            page_count: 1,
            preedit_mode: self.config.preedit,
            layout: self.config.layout,
            theme: self.config.theme,
            ..Frame::default()
        }
    }
}
