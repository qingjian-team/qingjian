//! 分页条的分层窗口：主窗拥有它，鼠标点击经回调交回工人线程。

use std::cell::Cell;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    GetDC, HDC, ReleaseDC, ScreenToClient, SetBkMode, TRANSPARENT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GWLP_USERDATA, GetWindowLongPtrW, HTCLIENT,
    HTTRANSPARENT, IDC_ARROW, LoadCursorW, MA_NOACTIVATE, SW_HIDE, SW_SHOWNA, SetWindowLongPtrW,
    ShowWindow, WM_LBUTTONUP, WM_MOUSEACTIVATE, WM_NCHITTEST, WNDCLASSEXW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{PCWSTR, Result, w};

use super::{PageClicks, state::PagingState};
use crate::ui::candidates::{theme::Theme, view};
use crate::ui::layered::{self, Layered};
use crate::ui::window_class::WindowClass;

const CLASS_NAME: PCWSTR = w!("QingjianPagingWindow");
static CLASS: WindowClass = WindowClass::new();

pub(in crate::ui::candidates) struct PagingWindow {
    hwnd: HWND,

    state: Box<PagingState>,
}

impl PagingWindow {
    pub fn new(owner: HWND, on_page: PageClicks) -> Result<Self> {
        CLASS.ensure(|| WNDCLASSEXW {
            lpfnWndProc: Some(wndproc),
            hInstance: crate::ui::module_handle(),
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default(),
            lpszClassName: CLASS_NAME,
            ..Default::default()
        })?;
        let state = Box::new(PagingState {
            on_page,
            page: Cell::new(0),
            pages: Cell::new(1),
            content: Cell::new(RECT::default()),
            button_width: Cell::new(0),
        });
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                CLASS_NAME,
                w!("青简翻页"),
                WS_POPUP,
                0,
                0,
                0,
                0,
                Some(owner),
                None,
                Some(crate::ui::module_handle()),
                None,
            )?
        };
        // Box 的地址随窗口存活；Drop 先销毁 HWND，再释放状态。
        unsafe {
            SetWindowLongPtrW(
                hwnd,
                GWLP_USERDATA,
                (&*state as *const PagingState) as isize,
            )
        };
        Ok(Self { hwnd, state })
    }

    pub fn size(&self, theme: &Theme, page: usize, pages: usize) -> (i32, i32) {
        if pages <= 1 {
            return (0, 0);
        }
        let hdc = unsafe { GetDC(Some(self.hwnd)) };
        let text = view::measure(hdc, theme.index_font, &format!("{}/{}", page + 1, pages));
        unsafe { ReleaseDC(Some(self.hwnd), hdc) };
        let button = theme.padding * 4;
        (
            text.cx + theme.padding * 2 + button * 2,
            (text.cy + theme.row_padding * 2).max(button),
        )
    }

    pub fn show(&self, position: (i32, i32), theme: &Theme, dpi: u32, page: usize, pages: usize) {
        let size = self.size(theme, page, pages);
        if size.0 == 0 {
            self.hide();
            return;
        }
        let margin = layered::shadow_margin(dpi);
        self.state.page.set(page);
        self.state.pages.set(pages);
        self.state.button_width.set(theme.padding * 4);
        self.state.content.set(RECT {
            left: margin,
            top: margin,
            right: margin + size.0,
            bottom: margin + size.1,
        });
        let result = layered::composite(
            self.hwnd,
            &Layered {
                content: size,
                margin,
                win_pos: (position.0 - margin, position.1 - margin),
                win_size: (size.0 + margin * 2, size.1 + margin * 2),
                background: theme.background,
                corner_radius: theme.corner_radius,
                paint: &|hdc, rect| self.paint(hdc, rect, theme),
            },
        );
        if result.is_ok() {
            let _ = unsafe { ShowWindow(self.hwnd, SW_SHOWNA) };
        } else {
            self.hide();
        }
    }

    fn paint(&self, hdc: HDC, rect: RECT, theme: &Theme) {
        unsafe { SetBkMode(hdc, TRANSPARENT) };
        let page = self.state.page.get();
        let pages = self.state.pages.get();
        let button = self.state.button_width.get();
        for (text, x, enabled) in [
            ("‹", 0, page > 0),
            ("›", rect.right - button, page + 1 < pages),
        ] {
            let size = view::measure(hdc, theme.symbol_font, text);
            view::draw_text(
                hdc,
                theme.symbol_font,
                if enabled {
                    theme.text_color
                } else {
                    theme.index_color
                },
                x + (button - size.cx) / 2,
                (rect.bottom - size.cy) / 2,
                text,
            );
        }
        let text = format!("{}/{}", page + 1, pages);
        let size = view::measure(hdc, theme.index_font, &text);
        view::draw_text(
            hdc,
            theme.index_font,
            theme.index_color,
            (rect.right - size.cx) / 2,
            (rect.bottom - size.cy) / 2,
            &text,
        );
    }

    pub fn hide(&self) {
        self.state.content.set(RECT::default());
        let _ = unsafe { ShowWindow(self.hwnd, SW_HIDE) };
    }
}

impl Drop for PagingWindow {
    fn drop(&mut self) {
        let _ = unsafe { DestroyWindow(self.hwnd) };
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let pointer = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const PagingState;
    if !pointer.is_null() {
        // 状态由 PagingWindow 持有，窗口过程只在所属 UI 线程读取。
        let state = unsafe { &*pointer };
        match msg {
            WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE as isize),
            WM_NCHITTEST => {
                let mut point = POINT {
                    x: (lparam.0 & 0xFFFF) as i16 as i32,
                    y: ((lparam.0 >> 16) & 0xFFFF) as i16 as i32,
                };
                if !unsafe { ScreenToClient(hwnd, &mut point) }.as_bool() {
                    return LRESULT(HTTRANSPARENT as isize);
                }
                return if state.contains(point.x, point.y) {
                    LRESULT(HTCLIENT as isize)
                } else {
                    LRESULT(HTTRANSPARENT as isize)
                };
            }
            WM_LBUTTONUP => {
                let x = (lparam.0 & 0xFFFF) as i16 as i32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
                if let Some(step) = state.step_at(x, y) {
                    (state.on_page)(step);
                }
                return LRESULT(0);
            }
            _ => {}
        }
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}
