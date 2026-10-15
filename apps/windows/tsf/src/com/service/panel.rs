//! 符号面板从悬浮条或任务栏图标菜单打开时，这个应用可能没在组句：按 Server 随 `ModeSync` 带来的状态
//! 先当成在组句，键才会交给 Server、轮询才会跑；上屏要用的文档上下文取焦点文档的顶层上下文。

use qingjian_platform::protocol::IndicatorState;

use super::TextService_Impl;
use crate::com::log::log;

impl TextService_Impl {
    /// 记下右键菜单的开关状态；面板开着而本地没在组句就进组句态。
    pub(super) fn apply_indicator_state(&self, state: IndicatorState) {
        self.indicator_state.set(state);
        if !state.symbol_panel || self.shared.composing() {
            return;
        }
        let context = self
            .thread_mgr
            .borrow()
            .as_ref()
            .and_then(|thread_mgr| unsafe { thread_mgr.GetFocus().ok()?.GetTop().ok() });
        if let Some(context) = context {
            self.shared.set_last_context(Some(context));
        }
        log("符号面板已打开，按键交给 Server");
        self.shared.set_composing(true);
    }
}
