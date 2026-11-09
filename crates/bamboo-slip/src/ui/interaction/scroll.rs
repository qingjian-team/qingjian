//! 滚动：滚轮 / 触控板的增量交给指针下面、还能往那个方向滚的最里层滚动区；每帧按新的内容大小把滚动位置夹回范围里。

use std::collections::HashMap;

use taffy::NodeId;

use super::Interaction;
use crate::ui::{Key, Response, ScrollExtent, ScrollbarTrack};

/// 一条轴上要滚多少才能露出元素；比视野长的只保证开头看得见。
fn reveal_delta(start: f32, length: f32, view_start: f32, view_length: f32, margin: f32) -> f32 {
    if start < view_start + margin || length > view_length - margin * 2.0 {
        start - (view_start + margin)
    } else if start + length > view_start + view_length - margin {
        start + length - (view_start + view_length - margin)
    } else {
        0.0
    }
}

impl<M> Interaction<M> {
    /// 指针按在滚动条轨道上就开始拖：按在滑块上抓住原处，按在别处让滑块中间跳过来。返回是否按在轨道上。
    pub(super) fn press_scrollbar(&mut self) -> bool {
        let Some(key) = self.hovered.first().copied() else {
            return false;
        };
        let Some(track) = self
            .element(key)
            .and_then(|element| element.interact.scrollbar)
        else {
            return false;
        };
        let Some((along, viewport, max)) = self.scrollbar_place(track) else {
            return false;
        };
        let offset = self.scroll_offset(track.scroller);
        let current = if track.horizontal { offset.0 } else { offset.1 };
        let start = track.thumb_start(current, max, viewport);
        self.scrollbar_grab = if (start..=start + track.thumb).contains(&along) {
            along - start
        } else {
            track.thumb / 2.0
        };
        self.pressed = Some(key);
        self.drag_scrollbar();
        true
    }

    /// 正在拖滚动条时按指针位置滚；返回滚动位置是否变了。
    pub(super) fn drag_scrollbar(&mut self) -> bool {
        let Some(track) = self
            .pressed
            .and_then(|key| self.element(key))
            .and_then(|element| element.interact.scrollbar)
        else {
            return false;
        };
        let Some((along, viewport, max)) = self.scrollbar_place(track) else {
            return false;
        };
        let target = track.offset_at(along, self.scrollbar_grab, max, viewport);
        let old = self.scroll_offset(track.scroller);
        let new = if track.horizontal {
            (target, old.1)
        } else {
            (old.0, target)
        };
        if new == old {
            return false;
        }
        self.offsets.insert(track.scroller, new);
        self.scroll_generation += 1;
        true
    }

    /// 指针沿滚动方向离视口起点多远、视口多长、最远能滚多少。指针移出窗口时没有。
    fn scrollbar_place(&self, track: ScrollbarTrack) -> Option<(f32, f32, f32)> {
        let (x, y) = self.pointer?;
        let (left, top, width, height) = self.element(track.scroller)?.rect();
        let (max_x, max_y) = self.scroll_extent(track.scroller)?.max();
        Some(if track.horizontal {
            (x - left, width, max_x)
        } else {
            (y - top, height, max_y)
        })
    }

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

    /// 把 `key` 那个元素滚进它所在滚动区的视野（四周留 `margin` 像素）；已经看得见就不动。返回滚动位置是否变了。
    pub fn reveal(&mut self, key: Key, margin: f32) -> bool {
        let Some(&start) = self.index.get(&key) else {
            return false;
        };
        let (left, top, width, height) = self.elements[start].rect();
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
            let (view_left, view_top, view_width, view_height) = element.rect();
            let old = self.scroll_offset(scroller);
            let (max_x, max_y) = extent.max();
            let new = (
                (old.0 + reveal_delta(left, width, view_left, view_width, margin))
                    .clamp(0.0, max_x),
                (old.1 + reveal_delta(top, height, view_top, view_height, margin))
                    .clamp(0.0, max_y),
            );
            if new == old {
                return false;
            }
            self.offsets.insert(scroller, new);
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
            // 只能横着滚的区域：普通鼠标只有竖滚轮，也让它横着滚
            let dx = if max_y == 0.0 && dx == 0.0 { dy } else { dx };
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
