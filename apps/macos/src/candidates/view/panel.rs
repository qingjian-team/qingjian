//! 符号面板在候选视图里的接线：一帧交给 [`PanelPainter`] 画成位图，点击与拖选按它给的区域认。
//! 不管候选窗选的是哪种画法，面板都由青简渲染器画；第一次打开面板时才建渲染器。

use objc2::DefinedClass;
use objc2_app_kit::NSEvent;
use objc2_foundation::NSPoint;
use qingjian_render::PanelFrame;

use super::CandidateView;
use crate::candidates::PanelMouse;
use crate::candidates::bitmap::PanelPainter;
use crate::candidates::bounds::ViewBounds;

/// 面板一行格子多高（点），触控板滚动按它折算行数；与青简绿主题 `panel.cell` 一致。
const ROW_HEIGHT: f64 = 46.0;

impl CandidateView {
    pub(super) fn panel_row_height(&self) -> f64 {
        ROW_HEIGHT
    }

    /// 画好面板，返回视图该有的尺寸与内容区；渲染器建不起来时为 `None`。
    pub(super) fn set_panel_frame(&self, frame: &PanelFrame) -> Option<ViewBounds> {
        let (dark, scale) = (self.is_dark(), self.backing_scale());
        let mut painter = self.ivars().panel.borrow_mut();
        if painter.is_none() {
            *painter = PanelPainter::new(self.ivars().render_theme.borrow().clone());
        }
        painter
            .as_mut()
            .map(|painter| painter.set_frame(frame, dark, scale))
    }

    pub(super) fn draw_panel(&self) {
        let (dark, scale) = (self.is_dark(), self.backing_scale());
        if let Some(painter) = &mut *self.ivars().panel.borrow_mut() {
            painter.draw(dark, scale);
        }
    }

    /// 翻译框里按住拖动：选区从按下的地方拉到鼠标下面的字；挪到原文以外的地方时选区不变。
    pub(super) fn panel_dragged(&self, event: &NSEvent) {
        let Some(last) = self.ivars().drag.get() else {
            return;
        };
        let point = self.convertPoint_fromView(event.locationInWindow(), None);
        let Some(PanelMouse::Caret { position, .. }) = self.panel_hit(point, false) else {
            return;
        };
        if position == last {
            return;
        }
        self.ivars().drag.set(Some(position));
        if let Some(handler) = &*self.ivars().on_panel_mouse.borrow() {
            handler(PanelMouse::Caret {
                position,
                extend: true,
            });
        }
    }

    /// 点中的是哪一格、哪个胶囊或标签；空白处返回 `None`。
    pub(super) fn panel_hit(&self, point: NSPoint, double: bool) -> Option<PanelMouse> {
        self.ivars().panel.borrow().as_ref()?.hit(point, double)
    }
}
