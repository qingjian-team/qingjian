//! 组句按键账（见 [`CompositionTrace`]）：每条消息 / 插件事件前后看组句开始还是结束，结束时记一行。

use qingjian_platform::logs::CompositionTrace;
use qingjian_platform::protocol::linux::LinuxRequest;
use qingjian_platform::protocol::{ClientMessage, ServerMessage};

use super::Router;

impl Router {
    pub(super) fn traced(&mut self, message: ClientMessage) -> Option<ServerMessage> {
        let label = CompositionTrace::message_label(&message);
        self.trace_around(label, |router| router.dispatch(message))
    }

    pub(super) fn traced_linux(&mut self, request: LinuxRequest) -> Option<ServerMessage> {
        let label = CompositionTrace::linux_label(&request.event);
        self.trace_around(label, |router| router.linux_event(request))
    }

    fn trace_around(
        &mut self,
        label: (Option<bool>, &str),
        handle: impl FnOnce(&mut Self) -> Option<ServerMessage>,
    ) -> Option<ServerMessage> {
        let before = !self.engine.composition().is_empty();
        let reply = handle(self);
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
