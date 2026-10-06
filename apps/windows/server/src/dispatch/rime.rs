//! 原生键处理保留「同时上屏并放行」语义；不经过青简拼音分流。
use super::Router;
use qingjian_platform::protocol::{KeyEvent, KeyOutcome, ServerMessage, SessionId};

impl Router {
    pub(super) fn handle_rime_key(&mut self, session: SessionId, event: KeyEvent) -> ServerMessage {
        let (key, mask) = event.rime_key();
        let (consumed, commit) = self
            .engine
            .process_rime_key(key, mask)
            .expect("Rime 已启用");
        self.english = self.engine.english_mode();
        self.recompose();
        let frame = self.self_drawn_frame();
        self.reconcile_candidates(&frame);
        self.reconcile_status();
        ServerMessage::KeyResult {
            session,
            commit,
            outcome: if consumed {
                KeyOutcome::Consumed
            } else {
                KeyOutcome::Passthrough
            },
            frame: self.current_frame(),
        }
    }
}
