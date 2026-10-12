//! 候选框自己的鼠标拖动状态；不激活窗口，不向游戏发送鼠标或键盘事件。

use super::positions::Positions;
use crate::ui::{game, monitor};
use std::cell::{Cell, RefCell};
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetCapture, ReleaseCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOSIZE, SetWindowPos,
};

pub(in crate::ui::candidates) struct DragState {
    pub(in crate::ui::candidates) app: RefCell<Option<String>>,

    pub(in crate::ui::candidates) header_bottom: Cell<i32>,

    pub(in crate::ui::candidates) content_offset: Cell<(i32, i32)>,

    /// 按下时的屏幕鼠标坐标、候选窗口矩形和游戏窗口矩形。
    gesture: Cell<Option<(POINT, RECT, RECT)>>,

    positions: RefCell<Positions>,
}

impl DragState {
    pub(in crate::ui::candidates) fn new() -> Self {
        Self {
            app: RefCell::new(None),
            header_bottom: Cell::new(0),
            content_offset: Cell::new((0, 0)),
            gesture: Cell::new(None),
            positions: RefCell::new(Positions::open(
                qingjian_platform::dirs::user_dir()
                    .map(|d| d.join("game-candidate-positions.toml")),
            )),
        }
    }

    pub(in crate::ui::candidates) fn anchor(
        &self,
        app: &str,
        bounds: RECT,
        fallback: RECT,
    ) -> RECT {
        self.positions
            .borrow()
            .get(app, bounds)
            .map_or(fallback, |p| RECT {
                left: p.x,
                top: p.y - 3,
                right: p.x + 1,
                bottom: p.y - 2,
            })
    }

    pub(in crate::ui::candidates) fn is_header(&self, client: POINT) -> bool {
        self.app.borrow().is_some()
            && client.x >= self.content_offset.get().0
            && client.y >= self.content_offset.get().1
            && client.y < self.header_bottom.get()
    }

    pub(in crate::ui::candidates) fn begin(
        &self,
        hwnd: HWND,
        client: POINT,
        screen: POINT,
    ) -> bool {
        if !self.is_header(client) {
            return false;
        }
        let Some(app) = self.app.borrow().clone() else {
            return false;
        };
        let Some(bounds) = game::foreground_bounds(&app) else {
            return false;
        };
        let mut window = RECT::default();
        if unsafe { GetWindowRect(hwnd, &mut window) }.is_err() {
            return false;
        }
        self.gesture.set(Some((screen, window, bounds)));
        unsafe { SetCapture(hwnd) };
        true
    }

    pub(in crate::ui::candidates) fn update(&self, hwnd: HWND, screen: POINT) {
        let Some((start, window, bounds)) = self.gesture.get() else {
            return;
        };
        let Some(app) = self.app.borrow().clone() else {
            return;
        };
        let work = monitor::work_area_near(POINT {
            x: bounds.left + (bounds.right - bounds.left) / 2,
            y: bounds.top + (bounds.bottom - bounds.top) / 2,
        });
        let left = bounds.left.max(work.left);
        let top = bounds.top.max(work.top);
        let right = bounds.right.min(work.right).max(left);
        let bottom = bounds.bottom.min(work.bottom).max(top);
        let x = (window.left + screen.x - start.x)
            .clamp(left, (right - (window.right - window.left)).max(left));
        let y = (window.top + screen.y - start.y)
            .clamp(top, (bottom - (window.bottom - window.top)).max(top));
        if unsafe {
            SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOACTIVATE,
            )
        }
        .is_ok()
        {
            let offset = self.content_offset.get();
            super::super::click::move_origin((x + offset.0, y + offset.1));
            self.positions.borrow_mut().set(
                &app,
                POINT {
                    x: x + offset.0,
                    y: y + offset.1,
                },
                bounds,
            );
        }
    }

    pub(in crate::ui::candidates) fn finish(&self, hwnd: HWND) {
        if self.gesture.take().is_some()
            && let Err(error) = self.positions.borrow().save()
        {
            tracing::warn!(%error, "游戏候选位置暂未保存，本次运行仍可使用");
        }
        if unsafe { GetCapture() } == hwnd {
            let _ = unsafe { ReleaseCapture() };
        }
    }
}
