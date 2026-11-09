//! 组句按键账（见 [`CompositionTrace`]）：每条消息前后看组句开始还是结束，结束时记一行。

use qingjian_platform::logs::CompositionTrace;
use qingjian_platform::protocol::{ClientMessage, ServerMessage};

use super::Router;

impl Router {
    pub(super) fn traced(&mut self, message: ClientMessage) -> Option<ServerMessage> {
        let label = CompositionTrace::message_label(&message);
        let before = !self.engine.composition().is_empty();
        let reply = self.dispatch(message);
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
