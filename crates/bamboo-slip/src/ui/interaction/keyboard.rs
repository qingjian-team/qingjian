//! 键盘事件：Tab / Shift+Tab 按画序移焦点；其余按键先给焦点元素、再往上冒，没人处理时空格 / 回车触发焦点元素的点击。
//! 文字输入只给拿着焦点、收文字的元素，不往上冒。

use super::Interaction;
use crate::ui::{KeyCode, KeyPress, Response, TextInput};

impl<M: Clone> Interaction<M> {
    pub(super) fn key(&mut self, press: &KeyPress) -> Response<M> {
        let mut response = Response::new();
        if press.key == KeyCode::Tab && !press.modifiers.command && !press.modifiers.alt {
            response.changed = self.move_focus(!press.modifiers.shift, &mut response);
            return response;
        }
        let focused = self.focused.and_then(|key| self.index.get(&key).copied());
        // 没有焦点时交给最外层的元素（全局快捷键登记在根上）
        let chain: Vec<usize> = match focused {
            Some(position) => self.chain(Some(position)).collect(),
            None => (0..self.elements.len())
                .rev()
                .filter(|&position| self.elements[position].parent.is_none())
                .collect(),
        };
        for position in chain {
            let interact = &self.elements[position].interact;
            if interact.disabled {
                continue;
            }
            if let Some(message) = interact.key.as_ref().and_then(|handler| handler(press)) {
                response.messages.push(message);
                return response;
            }
        }
        if matches!(press.key, KeyCode::Enter | KeyCode::Space)
            && let Some(message) = focused
                .map(|position| &self.elements[position].interact)
                .filter(|interact| interact.clickable())
                .and_then(|interact| interact.click.clone())
        {
            response.messages.push(message);
        }
        response
    }

    pub(super) fn text(&mut self, input: &TextInput) -> Response<M> {
        let mut response = Response::new();
        if let Some(message) = self
            .focused
            .and_then(|key| self.element(key))
            .filter(|element| element.interact.accepts_text())
            .and_then(|element| element.interact.text.as_ref())
            .and_then(|handler| handler(input))
        {
            response.messages.push(message);
        }
        response
    }

    /// 焦点移到下一个（或上一个）能拿焦点的元素，到头绕回；返回是否变了。
    fn move_focus(&mut self, forward: bool, response: &mut Response<M>) -> bool {
        let candidates: Vec<usize> = (0..self.elements.len())
            .filter(|&position| self.elements[position].interact.takes_focus())
            .collect();
        if candidates.is_empty() {
            return false;
        }
        let current = self
            .focused
            .and_then(|key| self.index.get(&key))
            .and_then(|position| {
                candidates
                    .iter()
                    .position(|candidate| candidate == position)
            });
        let next = match (current, forward) {
            (None, true) => 0,
            (None, false) => candidates.len() - 1,
            (Some(at), true) => (at + 1) % candidates.len(),
            (Some(at), false) => (at + candidates.len() - 1) % candidates.len(),
        };
        let key = self.elements[candidates[next]].key;
        let moved = self.set_focus(Some(key), response);
        let scrolled = self.reveal(key, 0.0);
        let changed = moved || scrolled || !self.focus_visible;
        self.focus_visible = true;
        changed
    }
}
