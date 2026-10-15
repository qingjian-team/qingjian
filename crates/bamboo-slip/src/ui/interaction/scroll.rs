//! 滚动：滚轮 / 触控板的增量交给指针下面、还能往那个方向滚的最里层滚动区；每帧按新的内容大小把滚动位置夹回范围里。

use std::collections::HashMap;

use taffy::NodeId;

use super::Interaction;
use crate::ui::{Key, Response, ScrollExtent};

impl<M> Interaction<M> {
    /// 滚动区当前的滚动位置（像素）；建树时按它把内容往上挪。
    pub fn scroll_offset(&self, key: Key) -> (f32, f32) {
        self.offsets.get(&key).copied().unwrap_or_default()
    }

    /// 滚动区上一帧的视口与内容大小；第一帧还没有。
    pub fn scroll_extent(&self, key: Key) -> Option<ScrollExtent> {
        self.extents.get(&key).copied()
    }

    /// 滚动位置变过几次，见字段说明。
    pub fn scroll_generation(&self) -> u64 {
        self.scroll_generation
    }

    /// 把 `key` 那个元素滚进它所在滚动区的视野（上下留 `margin` 像素）；已经看得见就不动。返回滚动位置是否变了。
    pub fn reveal(&mut self, key: Key, margin: f32) -> bool {
        let Some(&start) = self.index.get(&key) else {
            return false;
        };
        let (_, top, _, height) = self.elements[start].rect();
        let chain: Vec<usize> = self.chain(Some(start)).skip(1).collect();
        for position in chain {
            let element = &self.elements[position];
            let Some(extent) = self
                .extents
                .get(&element.key)
                .filter(|_| element.interact.scroll_content.is_some())
            else {
                continue;
            };
            let scroller = element.key;
            let (_, view_top, _, view_height) = element.rect();
            let old = self.scroll_offset(scroller);
            // 比视野高的只保证顶上看得见
            let delta = if top < view_top + margin || height > view_height - margin * 2.0 {
                top - (view_top + margin)
            } else if top + height > view_top + view_height - margin {
                top + height - (view_top + view_height - margin)
            } else {
                0.0
            };
            let new_y = (old.1 + delta).clamp(0.0, extent.max().1);
            if new_y == old.1 {
                return false;
            }
            self.offsets.insert(scroller, (old.0, new_y));
            self.scroll_generation += 1;
            return true;
        }
        false
    }

    /// 量出这一帧各滚动区的大小，把超出范围的滚动位置夹回来；返回滚动位置是否因此变了。
    pub(super) fn measure_scrolls(
        &mut self,
        boxes: &HashMap<NodeId, (f32, f32, f32, f32)>,
    ) -> bool {
        self.extents.clear();
        let mut changed = false;
        for element in &self.elements {
            let Some(content) = element.interact.scroll_content else {
                continue;
            };
            let Some(&(_, _, width, height)) = boxes.get(&content) else {
                continue;
            };
            let (_, _, viewport_width, viewport_height) = element.rect();
            let extent = ScrollExtent {
                viewport: (viewport_width, viewport_height),
                content: (width, height),
            };
            self.extents.insert(element.key, extent);
            if let Some(offset) = self.offsets.get_mut(&element.key) {
                let (max_x, max_y) = extent.max();
                let clamped = (offset.0.clamp(0.0, max_x), offset.1.clamp(0.0, max_y));
                if clamped != *offset {
                    *offset = clamped;
                    changed = true;
                }
            }
        }
        if changed {
            self.scroll_generation += 1;
        }
        changed
    }

    pub(super) fn scroll(&mut self, dx: f32, dy: f32) -> Response<M> {
        let mut response = Response::new();
        let start = self
            .hovered
            .first()
            .and_then(|key| self.index.get(key).copied());
        let chain: Vec<usize> = self.chain(start).collect();
        for position in chain {
            let element = &self.elements[position];
            let Some(extent) = self
                .extents
                .get(&element.key)
                .filter(|_| element.interact.scroll_content.is_some())
            else {
                continue;
            };
            let key = element.key;
            let (max_x, max_y) = extent.max();
            let old = self.scroll_offset(key);
            let new = (
                (old.0 + dx).clamp(0.0, max_x),
                (old.1 + dy).clamp(0.0, max_y),
            );
            if new != old {
                self.offsets.insert(key, new);
                self.scroll_generation += 1;
                response.changed = true;
                return response;
            }
        }
        response
    }
}
