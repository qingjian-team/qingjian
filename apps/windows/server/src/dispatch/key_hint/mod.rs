//! 键位提示：按 `[general] key_hint` 决定这一帧带不带键盘，开着五笔带字根图（混输也是），否则带双拼键位。
//! 键盘怎么画、摆在哪由 UI 线程管（`ui::candidates::key_hint`），与 macOS 壳 `host::presenting::key_hint` 同一套规则。

mod content;
mod state;
mod view;

use qingjian_platform::KeyHintMode;

pub use self::content::KeyHintContent;
pub(super) use self::state::KeyHintState;
pub use self::view::KeyHintView;
use super::Router;

impl Router {
    /// 这一帧的键位提示；不该画时为 `None`。组句结束（缓冲区空了）时清掉「卡住时显示」的状态。
    pub(super) fn key_hint_view(&mut self) -> Option<KeyHintView> {
        let composing = self.config.key_hint != KeyHintMode::Off
            && self.translation.is_none()
            && !self.engine.composition().is_empty();
        let chart = composing.then(|| self.engine.root_chart()).flatten();
        let hint = (composing && chart.is_none())
            .then(|| self.engine.key_hint())
            .flatten();
        let content = match (chart, hint) {
            (Some(chart), _) => KeyHintContent::Roots(chart),
            (None, Some(hint)) => KeyHintContent::Shuangpin(hint),
            (None, None) => {
                self.key_hint = KeyHintState::default();
                return None;
            }
        };
        // 字根图上没有「拼不成」的说法，只等停顿
        let stuck = matches!(&content, KeyHintContent::Shuangpin(hint) if hint.stuck);
        let show = match self.config.key_hint {
            KeyHintMode::Always => true,
            KeyHintMode::Auto => self
                .key_hint
                .update(self.engine.composition().text(), stuck),
            KeyHintMode::Off => false,
        };
        let (opacity, size) = self.config.key_hint_look;
        show.then(|| KeyHintView {
            content,
            opacity: f32::from(opacity) / 100.0,
            zoom: f32::from(size) / 100.0,
        })
    }

    /// 停顿计时到点：还在组句就重画一次，把键盘弹出来。
    pub(super) fn advance_key_hint(&mut self) {
        if self.key_hint.hesitated() && !self.engine.composition().is_empty() {
            let frame = self.self_drawn_frame();
            self.reconcile_candidates(&frame);
        }
    }
}
