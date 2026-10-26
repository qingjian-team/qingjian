//! 指针事件：移动更新悬停，按下捕获并给焦点，抬起时还在捕获的元素上就算一次点击；能拖动的元素按下与按住移动时都收到位置。

use super::Interaction;
use crate::ui::Response;

impl<M: Clone> Interaction<M> {
    pub(super) fn pointer_moved(&mut self, pointer: Option<(f32, f32)>) -> Response<M> {
        self.pointer = pointer;
        let mut response = Response::new();
        response.changed = self.rehover();
        self.drag(&mut response);
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
            .find(|element| element.interact.click.is_some() || element.interact.drag.is_some())
            .filter(|element| !element.interact.disabled)
            .map(|element| element.key);
        let focused = path
            .iter()
            .map(|&position| &self.elements[position])
            .find(|element| element.interact.takes_focus())
            .map(|element| element.key);
        let moved = self.set_focus(focused, &mut response);
        self.drag(&mut response);
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

    /// 捕获指针的元素能拖动时，按指针的横向位置要一条消息。
    fn drag(&self, response: &mut Response<M>) {
        let (Some(key), Some((x, _))) = (self.pressed, self.pointer) else {
            return;
        };
        let Some(element) = self
            .element(key)
            .filter(|element| element.interact.draggable())
        else {
            return;
        };
        let (left, _, width, _) = element.rect();
        let fraction = if width > 0.0 {
            ((x - left) / width).clamp(0.0, 1.0)
        } else {
            0.0
        };
        if let Some(message) = element
            .interact
            .drag
            .as_ref()
            .and_then(|drag| drag(fraction))
        {
            response.messages.push(message);
            response.changed = true;
        }
    }
}
