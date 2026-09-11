//! 候选窗口：不抢焦点、置顶的分层窗口，跟随光标，画拼音行与候选列表，四周柔和阴影。
//! 绘制在 [`view`]，绘制内容在 [`RenderData`]，一行的展示形态在 [`row`]，配色 / 字体在 [`theme`]。设计语言对齐 macOS 端。

mod render_data;
pub(crate) mod row;
pub(crate) mod theme;
pub(crate) mod view;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC};
use windows::Win32::UI::HiDpi::{GetDpiForSystem, GetDpiForWindow};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, IDC_ARROW, LoadCursorW, SW_HIDE, SW_SHOWNA,
    ShowWindow, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_POPUP,
};
use windows::core::{PCWSTR, Result, w};

use qingjian_platform::ThemeMode;
use qingjian_platform::protocol::Frame;

pub(crate) use self::render_data::RenderData;
use self::theme::Theme;
use super::layered::{self, Layered};
use super::monitor;
use super::window_class::WindowClass;

const CLASS_NAME: PCWSTR = w!("QingjianCandidateWindow");
static CLASS: WindowClass = WindowClass::new();

/// 光标行与候选窗之间的间隙（逻辑像素）。
const CARET_GAP: i32 = 2;

/// 按外观模式解析深浅；`System` 读系统主题。
pub(super) fn resolve_dark(mode: ThemeMode) -> bool {
    match mode {
        ThemeMode::Light => false,
        ThemeMode::Dark => true,
        ThemeMode::System => system_prefers_dark(),
    }
}

/// `HKCU\...\Themes\Personalize\AppsUseLightTheme` 为 0 是深色；读不到当浅色。
fn system_prefers_dark() -> bool {
    windows_registry::CURRENT_USER
        .open(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .and_then(|key| key.get_u32("AppsUseLightTheme"))
        .is_ok_and(|value| value == 0)
}

/// 候选窗口。内容经 `UpdateLayeredWindow` 一次贴上，窗口过程只走默认处理。
pub(crate) struct CandidateWindow {
    hwnd: HWND,

    /// 绘制内容。
    data: RefCell<RenderData>,

    /// 上次用的 DPI，变了重建字体。
    dpi: Cell<u32>,

    /// 上次解析出的深浅，变了重建配色。
    dark: Cell<bool>,
}

impl CandidateWindow {
    /// 建一个隐藏的候选窗口。
    pub(crate) fn new() -> Result<Self> {
        CLASS.ensure(|| WNDCLASSEXW {
            lpfnWndProc: Some(wndproc),
            hInstance: super::module_handle(),
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default(),
            lpszClassName: CLASS_NAME,
            ..Default::default()
        })?;
        let dpi = unsafe { GetDpiForSystem() }.max(96);
        let dark = resolve_dark(ThemeMode::default());
        let data = RefCell::new(RenderData::empty(Rc::new(Theme::new(dpi, dark))));
        // NOACTIVATE：显示时不抢应用焦点。
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                CLASS_NAME,
                w!("青简候选"),
                WS_POPUP,
                0,
                0,
                0,
                0,
                None,
                None,
                Some(super::module_handle()),
                None,
            )?
        };
        Ok(Self {
            hwnd,
            data,
            dpi: Cell::new(dpi),
            dark: Cell::new(dark),
        })
    }

    /// 刷新内容（不定位、不显示）。
    pub(crate) fn set_content(&self, frame: &Frame) {
        self.data.borrow_mut().set(frame);
    }

    /// 按光标矩形定位并显示：贴光标下方（放不下放上方），四周留出阴影。
    pub(crate) fn show(&self, anchor: RECT) {
        self.sync_theme();
        let margin = layered::shadow_margin(self.dpi.get());
        let content = self.preferred_size();
        if content.0 <= 0 || content.1 <= 0 {
            self.hide();
            return;
        }
        let (content_x, content_y) = place(anchor, content);
        let updated = {
            let data = self.data.borrow();
            layered::composite(
                self.hwnd,
                &Layered {
                    content,
                    margin,
                    win_pos: (content_x - margin, content_y - margin),
                    win_size: (content.0 + margin * 2, content.1 + margin * 2),
                    background: data.theme.background,
                    corner_radius: data.theme.corner_radius,
                    paint: &|hdc, client| view::paint(hdc, &data, client),
                },
            )
        };
        if updated.is_ok() {
            let _ = unsafe { ShowWindow(self.hwnd, SW_SHOWNA) };
        } else {
            self.hide();
        }
    }

    pub(crate) fn hide(&self) {
        let _ = unsafe { ShowWindow(self.hwnd, SW_HIDE) };
    }

    /// DPI 或深浅变了就重建主题；每次 `show` 前调。
    fn sync_theme(&self) {
        let dpi = match unsafe { GetDpiForWindow(self.hwnd) } {
            0 => self.dpi.get(),
            dpi => dpi,
        };
        let dark = resolve_dark(self.data.borrow().theme_mode);
        if dpi != self.dpi.get() || dark != self.dark.get() {
            self.data.borrow_mut().theme = Rc::new(Theme::new(dpi, dark));
            self.dpi.set(dpi);
            self.dark.set(dark);
        }
    }

    /// 内容需要的大小（不含阴影留白）。
    fn preferred_size(&self) -> (i32, i32) {
        let hdc = unsafe { GetDC(Some(self.hwnd)) };
        let size = view::preferred_size(hdc, &self.data.borrow());
        unsafe { ReleaseDC(Some(self.hwnd), hdc) };
        (size.cx, size.cy)
    }
}

impl Drop for CandidateWindow {
    fn drop(&mut self) {
        let _ = unsafe { DestroyWindow(self.hwnd) };
    }
}

/// 内容左上角：贴光标下方，放不下放上方，再放不下贴屏幕内；都夹在所在显示器工作区里。
fn place(anchor: RECT, content: (i32, i32)) -> (i32, i32) {
    let work = monitor::work_area_near(POINT {
        x: anchor.left,
        y: anchor.top,
    });
    let x = anchor
        .left
        .clamp(work.left, (work.right - content.0).max(work.left));
    let below = anchor.bottom + CARET_GAP;
    let above = anchor.top - CARET_GAP - content.1;
    let y = if below + content.1 <= work.bottom {
        below
    } else if above >= work.top {
        above
    } else {
        (work.bottom - content.1).max(work.top)
    };
    (x, y)
}

/// 分层窗口无需 `WM_PAINT`，全交默认处理。
unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}
