//! 组句按键账（见 [`CompositionTrace`]）：每条消息前后看组句开始还是结束，结束时记一行。

use qingjian_platform::logs::CompositionTrace;
use qingjian_platform::protocol::{ClientMessage, ServerMessage};

use super::{ConnectionId, Router};

impl Router {
    pub(super) fn traced_from(
        &mut self,
        conn: ConnectionId,
        message: ClientMessage,
    ) -> Option<ServerMessage> {
        // 会话归属不对的消息不往下走：组句账也不记，免得攻击者拿别人的会话号往账上写字。
        if !self.admits(conn, &message) {
            tracing::debug!(
                conn,
                session = ?message.session(),
                "message for a session of another connection, dropped"
            );
            return None;
        }
        let label = CompositionTrace::message_label(&message);
        let before = !self.engine.composition().is_empty();
        let reply = self.dispatch(conn, message);
        let after = !self.engine.composition().is_empty();
        let app = self.focused_app().map(str::to_owned);
        CompositionTrace::observe(
            &mut self.trace,
            (before, after),
            label,
            reply.as_ref(),
            app.as_deref(),
            self.config.preedit,
        );
        reply
    }
}
