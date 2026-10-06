//! Linux 首版协议消息与 Engine 按键分派。
use super::{Router, key::Effect, session::SessionInfo};
use qingjian_platform::protocol::{ClientMessage, KeyOutcome, PROTOCOL_VERSION, ServerMessage};

impl Router {
    pub(super) fn dispatch(&mut self, message: ClientMessage) -> Option<ServerMessage> {
        match message {
            ClientMessage::OpenSession {
                session,
                app,
                protocol,
            } => {
                if protocol != PROTOCOL_VERSION {
                    return None;
                }
                self.close_session(session);
                self.sessions.insert(session, SessionInfo::new(app));
                None
            }
            ClientMessage::Privacy { session, private } => {
                self.set_privacy(session, private);
                None
            }
            ClientMessage::Key { session, event } if self.sessions.contains_key(&session) => {
                self.ensure_focus(session);
                self.notice = None;
                let (commit, outcome) = match self.apply_key(&event) {
                    Effect::Changed(commit) => {
                        self.recompose();
                        (commit, KeyOutcome::Consumed)
                    }
                    Effect::Navigated => (None, KeyOutcome::Consumed),
                    Effect::Passthrough => (None, KeyOutcome::Passthrough),
                };
                let frame = self.current_frame();
                Some(ServerMessage::KeyResult {
                    session,
                    outcome,
                    commit,
                    frame,
                })
            }
            ClientMessage::Poll { session } if self.sessions.contains_key(&session) => {
                // 插件组句期间定时来问：顺带接模型、推进重排，回的帧就是最新顺序
                self.ensure_focus(session);
                self.tick();
                Some(ServerMessage::Update {
                    session,
                    frame: self.current_frame(),
                })
            }
            ClientMessage::Commit { session } if self.sessions.contains_key(&session) => {
                self.ensure_focus(session);
                let text = (!self.engine.composition().is_empty()).then(|| self.engine.take_raw());
                self.reset_composition();
                self.flush_learning();
                Some(ServerMessage::Committed { session, text })
            }
            ClientMessage::CloseSession { session } => {
                self.close_session(session);
                None
            }
            _ => None,
        }
    }
}
