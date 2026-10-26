//! 跨帧的交互状态与事件处理：上一帧的可交互元素（按画序）、指针位置、悬停路径、按下与焦点。

mod keyboard;
mod pointer;
mod scroll;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use taffy::NodeId;

use super::ScrollExtent;
use super::element::Element;
use super::{ElementState, Elements, Event, Key, Response};
use crate::error::RenderError;
use crate::scene::Scene;

pub struct Interaction<M> {
    /// 上一帧的可交互元素，按画序（先父后子），后面的盖在前面的上面。
    elements: Vec<Element<M>>,

    /// key → 元素下标；同一帧 key 重复时只认第一个。
    index: HashMap<Key, usize>,

    /// 指针位置（根坐标像素）；离开窗口为 `None`。
    pointer: Option<(f32, f32)>,

    /// 指针下的元素与它的可交互上级，从里到外。
    hovered: Vec<Key>,

    /// 按下时捕获指针的元素，抬起前一直归它。
    pressed: Option<Key>,

    focused: Option<Key>,

    /// 焦点是键盘带来的，画焦点环。
    focus_visible: bool,

    /// 各滚动区的滚动位置（像素，向右、向下为正），元素暂时不在树里也留着，回来时接着原来的位置。
    offsets: HashMap<Key, (f32, f32)>,

    /// 这一帧各滚动区能滚多远，与视口、内容的高度。
    extents: HashMap<Key, ScrollExtent>,

    /// 滚动位置每变一次加一：使用方拿它区分「内容被滚动挪了」与「布局真的变了」，前者不该触发位置过渡。
    scroll_generation: u64,

    /// 输入框光标正处在闪烁熄灭的半拍；由 [`crate::Runner`] 按时间翻。
    caret_hidden: bool,

    /// 应用要醒目标出的那一项，见 [`crate::App::spotlight`]。
    spotlight: Option<Key>,
}

impl<M> Default for Interaction<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> Interaction<M> {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            index: HashMap::new(),
            pointer: None,
            hovered: Vec::new(),
            pressed: None,
            focused: None,
            focus_visible: false,
            offsets: HashMap::new(),
            extents: HashMap::new(),
            scroll_generation: 0,
            caret_hidden: false,
            spotlight: None,
        }
    }

    /// 输入框的光标这一刻亮着（闪烁中熄灭的半拍为 `false`）。
    pub fn caret_visible(&self) -> bool {
        !self.caret_hidden
    }

    pub(crate) fn set_caret_visible(&mut self, visible: bool) {
        self.caret_hidden = !visible;
    }

    /// 应用要醒目标出的那一项。
    pub fn spotlight(&self) -> Option<Key> {
        self.spotlight
    }

    pub(crate) fn set_spotlight(&mut self, key: Option<Key>) {
        self.spotlight = key;
    }

    pub fn state(&self, key: Key) -> ElementState {
        let hovered = self.hovered.contains(&key);
        let focused = self.focused == Some(key);
        ElementState {
            hovered,
            pressed: hovered && self.pressed == Some(key),
            focused,
            focus_ring: focused && self.focus_visible,
        }
    }

    /// 元素上一帧在根坐标里的矩形（左、上、宽、高，像素）：弹出层定位、测试里找点击位置用。
    /// 这一帧所有元素的 key，按树序（自动化测试逐个点控件用）。
    pub fn keys(&self) -> impl Iterator<Item = Key> + '_ {
        self.elements.iter().map(|element| element.key)
    }

    pub fn rect(&self, key: Key) -> Option<(f32, f32, f32, f32)> {
        self.element(key).map(Element::rect)
    }

    /// 收下新一帧的元素（`scene` 已布局）。不在 `root` 下的元素丢掉；焦点、按下的元素没了（或焦点元素被禁用）就清掉，悬停按指针位置重新命中。
    /// 返回交互状态是否因此变了（布局挪动让指针换了元素），变了要再重建一次。
    pub fn attach(
        &mut self,
        scene: &Scene,
        root: NodeId,
        elements: Elements<M>,
    ) -> Result<bool, RenderError> {
        let mut pending: HashMap<NodeId, (Key, super::Interact<M>)> = elements
            .0
            .into_iter()
            .map(|(key, node, interact)| (node, (key, interact)))
            .collect();
        self.elements.clear();
        self.index.clear();
        let mut by_node: HashMap<NodeId, usize> = HashMap::new();
        let located = scene.paint_order(root)?;
        let boxes: HashMap<NodeId, (f32, f32, f32, f32)> = located
            .iter()
            .map(|place| (place.node, place.rect))
            .collect();
        for place in located {
            let node = place.node;
            let Some((key, interact)) = pending.remove(&node) else {
                continue;
            };
            let position = self.elements.len();
            match self.index.entry(key) {
                Entry::Vacant(entry) => {
                    entry.insert(position);
                }
                Entry::Occupied(_) => {
                    tracing::warn!(?key, "同一帧里 key 重复，命中与状态只认第一个");
                }
            }
            by_node.insert(node, position);
            self.elements.push(Element {
                key,
                interact,
                located: place,
                parent: None,
            });
        }
        for element in &mut self.elements {
            let mut current = scene.parent(element.node());
            while let Some(node) = current {
                if let Some(&position) = by_node.get(&node) {
                    element.parent = Some(position);
                    break;
                }
                current = scene.parent(node);
            }
        }

        let mut changed = false;
        if self.focused.is_some_and(|key| {
            !self
                .element(key)
                .is_some_and(|element| element.interact.takes_focus())
        }) {
            self.focused = None;
            changed = true;
        }
        if self
            .pressed
            .is_some_and(|key| !self.index.contains_key(&key))
        {
            self.pressed = None;
            changed = true;
        }
        changed |= self.measure_scrolls(&boxes);
        changed |= self.rehover();
        Ok(changed)
    }

    /// 处理一个输入事件。
    pub fn handle(&mut self, event: &Event) -> Response<M>
    where
        M: Clone,
    {
        match event {
            Event::PointerMoved { x, y } => self.pointer_moved(Some((*x, *y))),
            Event::PointerLeft => self.pointer_moved(None),
            Event::PointerDown => self.pointer_down(),
            Event::PointerUp => self.pointer_up(),
            Event::Key(press) => self.key(press),
            Event::Text(input) => self.text(input),
            Event::Scroll { dx, dy } => self.scroll(*dx, *dy),
        }
    }

    /// 拿着焦点、收文字的元素在根坐标里的矩形：宿主据此开关输入法、摆输入法的候选窗。
    pub fn text_target(&self) -> Option<(f32, f32, f32, f32)> {
        self.focused
            .and_then(|key| self.element(key))
            .filter(|element| element.interact.accepts_text())
            .map(Element::rect)
    }

    /// 拿焦点的元素要不要开输入法：秘密输入（API 密钥这类）关掉，免得正在打的字被输入法
    /// 组字、上报（云端输入法会把组字内容发给服务商）。关着时宿主仍把原始键交给字段。
    pub fn ime_allowed(&self) -> bool {
        self.focused
            .and_then(|key| self.element(key))
            .is_none_or(|element| !element.interact.secret)
    }

    /// 换焦点；离开的元素有失焦消息就放进 `response`。返回焦点是否变了。
    fn set_focus(&mut self, focused: Option<Key>, response: &mut Response<M>) -> bool
    where
        M: Clone,
    {
        if focused == self.focused {
            return false;
        }
        if let Some(message) = self
            .focused
            .and_then(|key| self.element(key))
            .and_then(|element| element.interact.blur.clone())
        {
            response.messages.push(message);
        }
        self.focused = focused;
        true
    }

    /// 按指针位置重新算悬停路径，返回是否变了。
    fn rehover(&mut self) -> bool {
        let hovered = match self.pointer {
            Some((x, y)) => self.hit(x, y),
            None => Vec::new(),
        };
        let changed = hovered != self.hovered;
        self.hovered = hovered;
        changed
    }

    /// 命中：画序最后（最上面）的、矩形包含这一点的元素，加上它的可交互上级，从里到外。
    fn hit(&self, x: f32, y: f32) -> Vec<Key> {
        let target = self
            .elements
            .iter()
            .rposition(|element| element.contains(x, y));
        self.chain(target)
            .map(|position| self.elements[position].key)
            .collect()
    }

    /// 从 `start` 往上的元素下标。
    fn chain(&self, start: Option<usize>) -> impl Iterator<Item = usize> + '_ {
        std::iter::successors(start, |&position| self.elements[position].parent)
    }

    fn element(&self, key: Key) -> Option<&Element<M>> {
        self.index
            .get(&key)
            .map(|&position| &self.elements[position])
    }
}
