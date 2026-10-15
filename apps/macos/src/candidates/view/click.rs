//! 鼠标点候选窗口：按位图渲染器给的点击区域认出点中的候选或译词，交给 host 注册的回调上屏。
//! AppKit 逐项绘制的候选没有点击区域，点了不动；符号面板自己认格子与标签（见 `panel.rs`）。

use objc2::DefinedClass;
use objc2_app_kit::NSEvent;
use qingjian_render::HitTarget;

use super::CandidateView;
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
            .and_then(|bitmap| bitmap.hit(point));
        let Some(target) = target else {
            return;
        };
        tracing::debug!(?target, "点击候选窗口");
        if let Some(handler) = &*self.ivars().on_click.borrow() {
            handler(target);
        }
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
