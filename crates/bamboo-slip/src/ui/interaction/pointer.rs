//! 指针事件：移动更新悬停，按下捕获并给焦点，抬起时还在捕获的元素上就算一次点击。

use super::Interaction;
use crate::ui::Response;

impl<M: Clone> Interaction<M> {
    pub(super) fn pointer_moved(&mut self, pointer: Option<(f32, f32)>) -> Response<M> {
        self.pointer = pointer;
        let mut response = Response::new();
        response.changed = self.rehover();
        response
    }

    pub(super) fn pointer_down(&mut self) -> Response<M> {
        let mut response = Response::new();
        let path: Vec<usize> = match self.hovered.first() {
            Some(&key) => self.chain(self.index.get(&key).copied()).collect(),
            None => Vec::new(),
        };
        // 最近的带点击消息的元素捕获指针；它被禁用就谁都不捕获，不往上漏给外层
        self.pressed = path
            .iter()
            .map(|&position| &self.elements[position])
            .find(|element| element.interact.click.is_some())
            .filter(|element| !element.interact.disabled)
            .map(|element| element.key);
        let focused = path
            .iter()
            .map(|&position| &self.elements[position])
            .find(|element| element.interact.takes_focus())
            .map(|element| element.key);
        let moved = self.set_focus(focused, &mut response);
        response.changed = self.pressed.is_some() || moved || self.focus_visible;
        self.focus_visible = false;
        response
    }

    pub(super) fn pointer_up(&mut self) -> Response<M> {
        let mut response = Response::new();
        let Some(key) = self.pressed.take() else {
            return response;
        };
        response.changed = true;
        if self.hovered.contains(&key)
            && let Some(message) = self
                .element(key)
                .filter(|element| element.interact.clickable())
                .and_then(|element| element.interact.click.clone())
        {
            response.messages.push(message);
        }
        response
    }
}
