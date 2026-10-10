//! 分页条的窗口过程状态，像素几何与绘制共用，边界按钮只收鼠标不翻页。

use std::cell::Cell;
use windows::Win32::Foundation::RECT;

use super::PageClicks;

pub(super) struct PagingState {
    pub on_page: PageClicks,

    pub page: Cell<usize>,

    pub pages: Cell<usize>,

    pub content: Cell<RECT>,

    pub button_width: Cell<i32>,
}

impl PagingState {
    pub fn contains(&self, x: i32, y: i32) -> bool {
        let rect = self.content.get();
        x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom
    }

    pub fn step_at(&self, x: i32, y: i32) -> Option<isize> {
        if !self.contains(x, y) {
            return None;
        }
        let rect = self.content.get();
        if x < rect.left + self.button_width.get() && self.page.get() > 0 {
            Some(-1)
        } else if x >= rect.right - self.button_width.get()
            && self.page.get() + 1 < self.pages.get()
        {
            Some(1)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PagingState;
    use std::cell::Cell;
    use windows::Win32::Foundation::RECT;

    #[test]
    fn page_buttons_share_content_geometry_and_stop_at_boundaries() {
        let state = PagingState {
            on_page: Box::new(|_| {}),
            page: Cell::new(0),
            pages: Cell::new(3),
            content: Cell::new(RECT {
                left: 16,
                top: 16,
                right: 116,
                bottom: 48,
            }),
            button_width: Cell::new(28),
        };
        assert_eq!(state.step_at(20, 20), None);
        assert_eq!(state.step_at(100, 20), Some(1));
        state.page.set(1);
        assert_eq!(state.step_at(20, 20), Some(-1));
        assert_eq!(state.step_at(100, 20), Some(1));
        assert_eq!(state.step_at(60, 20), None);
        assert_eq!(state.step_at(100, 48), None);
        assert_eq!(state.step_at(116, 20), None);
        assert_eq!(state.step_at(15, 20), None);
        state.page.set(2);
        assert_eq!(state.step_at(100, 20), None);
    }
}
