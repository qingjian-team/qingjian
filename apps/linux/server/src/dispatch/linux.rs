//! Linux 事件决策；Fcitx 插件只报告事实并应用返回值。
use super::{Router, key::Effect};
use crate::protocol::{DisplayIdentity, LinuxEvent, LinuxRequest};
use qingjian_platform::protocol::{KeyOutcome, ServerMessage, SessionId};

impl Router {
    pub(super) fn linux_event(&mut self, request: LinuxRequest) -> Option<ServerMessage> {
        let session = request.session;
        if !self.sessions.contains_key(&session) {
            return None;
        }
        let mut outcome = KeyOutcome::Passthrough;
        let mut commit = None;
        match request.event {
            LinuxEvent::Reset => {
                if self.focused == Some(session) {
                    self.end_translation();
                }
                self.discard_session(session);
            }
            LinuxEvent::Capabilities(caps) => {
                let disabled = caps.password || caps.disabled;
                let private = disabled || caps.sensitive;
                if self.sessions[&session].capabilities != Some(caps) {
                    // 即使 Sensitive → Password 仍为 private，也必须丢弃全部旧输入。
                    self.discard_session(session);
                    self.set_privacy(session, private);
                    let info = self.sessions.get_mut(&session)?;
                    info.capabilities = Some(caps);
                    info.disabled = disabled;
                }
            }
            LinuxEvent::Focus { focused } => {
                let info = self.sessions.get_mut(&session)?;
                let was_active = info.active;
                info.active = focused;
                info.shift_pending = false;
                info.display_frame = None;
                info.last_frame = None;
                if !focused && self.focused == Some(session) {
                    self.end_translation();
                    self.engine.note_displayed(std::iter::empty());
                    self.engine.cancel_prediction();
                }
                if focused {
                    let was_focused = self.focused == Some(session);
                    self.ensure_focus(session);
                    if was_focused && !was_active {
                        self.request_current_prediction();
                    }
                }
            }
            LinuxEvent::Deactivate {
                focus_out,
                client_preedit,
                capability_changed,
            } => {
                if self.focused == Some(session) {
                    self.end_translation();
                }
                if capability_changed || self.sessions[&session].disabled {
                    self.discard_session(session);
                } else {
                    self.ensure_focus(session);
                    let text =
                        (!self.engine.composition().is_empty()).then(|| self.engine.take_raw());
                    if !(focus_out && client_preedit) {
                        commit = text;
                    }
                    self.reset_composition();
                }
                let info = self.sessions.get_mut(&session)?;
                info.active = false;
                info.shift_pending = false;
                info.display_frame = None;
                info.last_frame = None;
                self.flush_learning();
            }
            LinuxEvent::Key {
                mut event,
                release,
                surrounding,
                selection_supported,
            } => {
                if self.sessions[&session].disabled
                    || self.sessions[&session].capabilities.is_none()
                {
                    return Some(ServerMessage::KeyResult {
                        session,
                        outcome,
                        commit,
                        frame: Default::default(),
                    });
                }
                self.ensure_focus(session);
                self.notice = None;
                if !release && self.translation.is_some() {
                    let (outcome, commit) = self.review_translation(&event);
                    return Some(ServerMessage::KeyResult {
                        session,
                        outcome,
                        commit,
                        frame: self.current_frame(),
                    });
                }
                if !release
                    && selection_supported
                    && self.engine.composition().is_empty()
                    && self.engine.prediction_enabled()
                    && !self.sessions[&session].private
                    && self.sessions[&session].active
                    && self.matches_translate_combo(&event)
                {
                    return Some(self.request_selection(session));
                }
                if release && self.translation.is_some() {
                    return Some(ServerMessage::KeyResult {
                        session,
                        outcome,
                        commit,
                        frame: self.current_frame(),
                    });
                }
                let surrounding = (!self.sessions[&session].private)
                    .then_some(surrounding)
                    .flatten()
                    .map(Into::into);
                let info = self.sessions.get_mut(&session)?;
                let shift = event.virtual_key == 0x10;
                if release {
                    if shift && info.shift_pending {
                        info.shift_pending = false;
                        info.english = !info.english;
                        commit =
                            (!self.engine.composition().is_empty()).then(|| self.engine.take_raw());
                        self.reset_composition();
                        outcome = KeyOutcome::Consumed;
                    }
                } else {
                    info.shift_pending = shift && !event.modifiers.has_command_key();
                    event.modifiers.english_mode = info.english;
                    let effect = if shift {
                        Effect::Passthrough
                    } else {
                        self.apply_key(&event)
                    };
                    match effect {
                        Effect::Changed(text) => {
                            self.recompose(surrounding);
                            commit = text;
                            outcome = KeyOutcome::Consumed;
                        }
                        Effect::Navigated => outcome = KeyOutcome::Consumed,
                        Effect::Passthrough => {}
                    }
                }
            }
            LinuxEvent::Candidate { identity, index } => {
                if self.valid_panel_event(session, &identity) && index < self.config.page_size {
                    if self.translation.is_some() {
                        if index == 0 {
                            commit = self.translation.as_ref().and_then(|job| job.result.clone());
                            if commit.is_some() {
                                self.end_translation();
                            }
                        }
                        outcome = KeyOutcome::Consumed;
                    } else {
                        let offset = self.highlight / self.config.page_size * self.config.page_size;
                        commit = self.commit_index(offset + index);
                        if commit.is_some() {
                            self.recompose(None);
                            outcome = KeyOutcome::Consumed;
                        }
                    }
                }
            }
            LinuxEvent::Selection { request, text } => {
                outcome = self.receive_selection(session, request, text);
            }
            LinuxEvent::Page { identity, next } => {
                if self.valid_panel_event(session, &identity) {
                    self.page(if next { 1 } else { -1 });
                    outcome = KeyOutcome::Consumed;
                }
            }
        }
        let frame = if self.focused == Some(session) && !self.sessions[&session].disabled {
            self.current_frame()
        } else {
            Default::default()
        };
        Some(ServerMessage::KeyResult {
            session,
            outcome,
            commit,
            frame,
        })
    }

    pub(super) fn valid_panel_event(&self, session: SessionId, identity: &DisplayIdentity) -> bool {
        let Some(info) = self.sessions.get(&session) else {
            return false;
        };
        self.focused == Some(session)
            && info.active
            && !info.disabled
            && info.display_identity.as_ref() == Some(identity)
    }

    fn discard_session(&mut self, session: SessionId) {
        let info = self.sessions.get_mut(&session).expect("known session");
        info.shift_pending = false;
        info.display_frame = None;
        info.last_frame = None;
        info.composed = None;
        info.highlight = 0;
        info.navigated = false;
        if let Some(identity) = &mut info.display_identity {
            self.display_revision += 1;
            identity.revision = self.display_revision;
        }
        if self.focused == Some(session) {
            self.stop_rescoring();
            self.engine.discard_input();
            self.composed = None;
            self.sentence = None;
            self.notice = None;
            self.highlight = 0;
            self.navigated = false;
        } else {
            info.engine.discard_input();
        }
    }
}
