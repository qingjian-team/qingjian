//! 鼠标点候选窗口：位图渲染器直接给点击区域，AppKit 系统绘制按排版尺寸反算候选格；
//! 符号面板自己认格子与标签（见 `panel.rs`）。

use objc2::DefinedClass;
use objc2_app_kit::NSEvent;
use objc2_foundation::NSPoint;
use qingjian_platform::LayoutMode;
use qingjian_render::HitTarget;

use super::{CandidateView, HIGHLIGHT_INSET, INDEX_GAP};
use crate::candidates::PanelMouse;

/// 点中候选或译词时调的回调。
pub type ClickHandler = Box<dyn Fn(HitTarget)>;

/// 符号面板里点中格子或标签时调的回调。
pub type PanelMouseHandler = Box<dyn Fn(PanelMouse)>;

impl CandidateView {
    pub fn set_click_handler(&self, handler: ClickHandler) {
        *self.ivars().on_click.borrow_mut() = Some(handler);
    }

    pub fn set_panel_mouse_handler(&self, handler: PanelMouseHandler) {
        *self.ivars().on_panel_mouse.borrow_mut() = Some(handler);
    }

    pub(super) fn clicked(&self, event: &NSEvent) {
        let point = self.convertPoint_fromView(event.locationInWindow(), None);
        if self.ivars().frame.borrow().is_panel() {
            let hit = self.panel_hit(point, event.clickCount() >= 2);
            if let (Some(hit), Some(handler)) = (hit, &*self.ivars().on_panel_mouse.borrow()) {
                tracing::debug!(?hit, "点击符号面板");
                handler(hit);
            }
            return;
        }
        // 先放掉位图的借用：回调里上屏会重画这个视图
        let target = self
            .ivars()
            .bitmap
            .borrow()
            .as_ref()
            .and_then(|bitmap| bitmap.hit(point))
            .or_else(|| self.system_candidate_at(point).map(HitTarget::Candidate));
        let Some(target) = target else {
            return;
        };
        tracing::debug!(?target, "点击候选窗口");
        if let Some(handler) = &*self.ivars().on_click.borrow() {
            handler(target);
        }
    }

    /// AppKit 系统绘制没有位图渲染器提供的点击区域，按同一套排版尺寸反算候选格。
    pub(super) fn system_candidate_at(&self, point: NSPoint) -> Option<usize> {
        let frame = self.ivars().frame.borrow();
        let theme = self.theme();
        let body_y = theme.padding + self.top_line_size(&frame).1;
        if point.y < body_y {
            return None;
        }
        let offset = match self.ivars().layout.get() {
            LayoutMode::Vertical => {
                let row_height = self.columns(&frame.rows).row_height;
                (row_height > 0.0).then(|| ((point.y - body_y) / row_height) as usize)?
            }
            LayoutMode::Horizontal if frame.columns > 0 => {
                let cells = self.matrix_cells(&frame);
                if cells.row_height <= 0.0 {
                    return None;
                }
                let row = ((point.y - body_y) / cells.row_height) as usize;
                let origin = theme.padding + HIGHLIGHT_INSET;
                let x = point.x - origin;
                if x < 0.0 {
                    return None;
                }
                let mut left = 0.0;
                let mut column = None;
                for (index, width) in cells.column_widths.iter().enumerate() {
                    if x >= left && x <= left + width + HIGHLIGHT_INSET * 2.0 {
                        column = Some(index);
                        break;
                    }
                    left += width + theme.column_gap;
                }
                row * frame.columns + column?
            }
            LayoutMode::Horizontal => {
                let (items, row_height) = self.items(&frame.rows);
                if row_height <= 0.0 || point.y > body_y + row_height {
                    return None;
                }
                let mut left = theme.padding;
                let mut found = None;
                for (index, item) in items.iter().enumerate() {
                    let width =
                        item.index_width + INDEX_GAP + item.text_width + HIGHLIGHT_INSET * 2.0;
                    if point.x >= left && point.x <= left + width {
                        found = Some(index);
                        break;
                    }
                    left += width + theme.column_gap;
                }
                found?
            }
        };
        frame
            .rows
            .get(offset)
            .filter(|row| !row.text.is_empty())
            .map(|_| offset)
    }

    /// 面板里滚轮 / 触控板：按格子高度攒够一行滚一行；鼠标滚轮一格就是一行。
    pub(super) fn scrolled(&self, event: &NSEvent) {
        if !self.ivars().frame.borrow().is_panel() {
            return;
        }
        let delta = event.scrollingDeltaY();
        let rows = if event.hasPreciseScrollingDeltas() {
            let total = self.ivars().scroll_rest.get() + delta;
            let rows = (total / self.panel_row_height()).trunc();
            self.ivars()
                .scroll_rest
                .set(total - rows * self.panel_row_height());
            rows
        } else {
            delta.signum() * delta.abs().ceil()
        };
        // 往上滑（delta 为正）是看上面的内容，视口往上
        let rows = -rows as isize;
        if rows == 0 {
            return;
        }
        if let Some(handler) = &*self.ivars().on_panel_mouse.borrow() {
            handler(PanelMouse::Scroll(rows));
        }
    }
}
