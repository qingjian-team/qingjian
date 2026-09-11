//! 悬浮状态条：桌面上常驻、可拖动的三格浮窗 `[中 / 英][，。/ ,.][⚙]`，复用分层窗口合成器与候选窗口主题。
//!
//! 按下鼠标先 `DragDetect`：挪出拖动阈值就交给系统的移动循环（`WM_NCLBUTTONDOWN` + `HTCAPTION`），
//! 结束时 `WM_EXITSIZEMOVE` 报新位置；没挪就是点击，按 x 落进哪格。`WM_MOUSEACTIVATE` 回 `MA_NOACTIVATE` 不抢焦点。
//! 一格的规格在 [`cell`]，摆放与点击在 [`placement`]。

mod cell;
mod placement;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{GetDC, HDC, ReleaseDC, SetBkMode, TRANSPARENT};
use windows::Win32::UI::HiDpi::{GetDpiForSystem, GetDpiForWindow};
use windows::Win32::UI::Input::KeyboardAndMouse::{DragDetect, ReleaseCapture};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetCursorPos, HTCAPTION, HTCLIENT, IDC_HAND,
    LoadCursorW, MA_NOACTIVATE, SW_HIDE, SW_SHOWNA, SendMessageW, ShowWindow, WM_EXITSIZEMOVE,
    WM_LBUTTONDOWN, WM_MOUSEACTIVATE, WM_NCHITTEST, WM_NCLBUTTONDOWN, WNDCLASSEXW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{PCWSTR, Result, w};

use qingjian_platform::ThemeMode;

use self::cell::CellSpec;
use self::placement::{Placement, StatusAction};
use super::StatusEvents;
use super::candidates::resolve_dark;
use super::candidates::theme::Theme;
use super::candidates::view;
use super::layered::{self, Layered};
use super::monitor;
use super::window_class::WindowClass;
use crate::dispatch::StatusView;

const CLASS_NAME: PCWSTR = w!("QingjianStatusBar");
static CLASS: WindowClass = WindowClass::new();

/// 状态条与屏幕边缘的间隙（逻辑像素）。
const EDGE_GAP: i32 = 8;

thread_local! {
    /// 本线程活着的状态条：HWND → 摆放状态。窗口过程按 HWND 查，查不到（已析构）就忽略。
    static PLACEMENTS: RefCell<HashMap<isize, Rc<Placement>>> = RefCell::new(HashMap::new());
}

/// 悬浮状态条窗口。
pub(super) struct StatusBar {
    hwnd: HWND,

    /// 最近一次要显示的内容；还没显示过时为 `None`。
    data: RefCell<Option<StatusView>>,

    /// 按 DPI / 深浅造好的主题（复用候选窗口那套）。
    theme: RefCell<Rc<Theme>>,

    /// 上次用的 DPI，变了重建主题。
    dpi: Cell<u32>,

    /// 上次解析出的深浅，变了重建配色。
    dark: Cell<bool>,

    /// 摆放状态，与窗口过程共享。
    placement: Rc<Placement>,
}

impl StatusBar {
    /// 建一个隐藏的状态条窗口。
    pub(super) fn new(events: StatusEvents) -> Result<Self> {
        CLASS.ensure(|| WNDCLASSEXW {
            lpfnWndProc: Some(wndproc),
            hInstance: super::module_handle(),
            hCursor: unsafe { LoadCursorW(None, IDC_HAND) }.unwrap_or_default(),
            lpszClassName: CLASS_NAME,
            ..Default::default()
        })?;
        let dpi = unsafe { GetDpiForSystem() }.max(96);
        let dark = resolve_dark(ThemeMode::default());
        // NOACTIVATE：显示时不抢应用焦点。
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                CLASS_NAME,
                w!("青简状态条"),
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
        let placement = Rc::new(Placement::new(hwnd, layered::shadow_margin(dpi), events));
        PLACEMENTS.with(|map| map.borrow_mut().insert(hwnd.0 as isize, placement.clone()));
        Ok(Self {
            hwnd,
            data: RefCell::new(None),
            theme: RefCell::new(Rc::new(Theme::new(dpi, dark))),
            dpi: Cell::new(dpi),
            dark: Cell::new(dark),
            placement,
        })
    }

    /// 显示 / 更新：按记住的位置（首次用 `view.anchor`，都没有就右下角）摆放并重绘。
    pub(super) fn update(&self, view: StatusView) {
        if self.placement.pos.get().is_none() {
            self.placement.pos.set(view.anchor);
        }
        *self.data.borrow_mut() = Some(view);
        self.sync_theme();
        self.render();
    }

    pub(super) fn hide(&self) {
        let _ = unsafe { ShowWindow(self.hwnd, SW_HIDE) };
    }

    /// DPI 或深浅变了就重建主题。
    fn sync_theme(&self) {
        let dpi = match unsafe { GetDpiForWindow(self.hwnd) } {
            0 => self.dpi.get(),
            dpi => dpi,
        };
        let mode = self
            .data
            .borrow()
            .as_ref()
            .map(|view| view.theme)
            .unwrap_or_default();
        let dark = resolve_dark(mode);
        if dpi != self.dpi.get() || dark != self.dark.get() {
            *self.theme.borrow_mut() = Rc::new(Theme::new(dpi, dark));
            self.dpi.set(dpi);
            self.dark.set(dark);
        }
    }

    /// 三格从左到右：模式（品牌色）、标点（生效时品牌色，否则灰）、齿轮（灰）。
    fn cells(&self, theme: &Theme) -> Vec<CellSpec> {
        let data = self.data.borrow();
        let Some(view) = data.as_ref() else {
            return Vec::new();
        };
        let mode = if view.english {
            "英".to_owned()
        } else {
            match &view.scheme {
                Some(scheme) => format!("中 · {scheme}"),
                None => "中".to_owned(),
            }
        };
        let punctuation_active = view.full_width;
        vec![
            CellSpec {
                text: mode,
                font: theme.text_font,
                color: theme.cloud_color,
                action: StatusAction::ToggleMode,
            },
            CellSpec {
                text: if punctuation_active { "，。" } else { ",." }.to_owned(),
                font: theme.text_font,
                color: if punctuation_active {
                    theme.cloud_color
                } else {
                    theme.gloss_color
                },
                action: StatusAction::TogglePunctuation,
            },
            CellSpec {
                text: "\u{2699}".to_owned(),
                font: theme.symbol_font,
                color: theme.gloss_color,
                action: StatusAction::OpenSettings,
            },
        ]
    }

    /// 量各格、算内容尺寸、摆位置、合成贴上；顺带记下各格边界给点击用。
    fn render(&self) {
        let theme = self.theme.borrow().clone();
        let margin = layered::shadow_margin(self.dpi.get());
        self.placement.margin.set(margin);
        let cells = self.cells(&theme);
        let hdc = unsafe { GetDC(Some(self.hwnd)) };
        let sizes: Vec<SIZE> = cells
            .iter()
            .map(|cell| view::measure(hdc, cell.font, &cell.text))
            .collect();
        unsafe { ReleaseDC(Some(self.hwnd), hdc) };
        let line = sizes.iter().map(|size| size.cy).max().unwrap_or(0);
        // 每格：左右各一个 padding；格间一条细线。
        let widths: Vec<i32> = sizes
            .iter()
            .map(|size| size.cx + theme.padding * 2)
            .collect();
        let content = (widths.iter().sum::<i32>(), line + theme.padding);
        if content.0 <= 0 || content.1 <= 0 || cells.is_empty() {
            self.hide();
            return;
        }
        let mut right = 0;
        let bounds: Vec<(i32, StatusAction)> = cells
            .iter()
            .zip(&widths)
            .map(|(cell, width)| {
                right += width;
                (right, cell.action)
            })
            .collect();
        *self.placement.cells.borrow_mut() = bounds;

        let anchor = self
            .placement
            .pos
            .get()
            .unwrap_or_else(|| default_anchor(content, margin));
        let anchor = clamp_anchor(anchor, content, margin);
        self.placement.pos.set(Some(anchor));

        let separator = theme.pos_color;
        let inset = theme.padding / 2;
        let updated = layered::composite(
            self.hwnd,
            &Layered {
                content,
                margin,
                win_pos: (anchor.0 - margin, anchor.1 - margin),
                win_size: (content.0 + margin * 2, content.1 + margin * 2),
                background: theme.background,
                corner_radius: theme.corner_radius,
                paint: &|hdc, client| {
                    unsafe { SetBkMode(hdc, TRANSPARENT) };
                    paint_cells(hdc, client, &cells, &sizes, &widths, separator, inset);
                },
            },
        );
        if updated.is_ok() {
            let _ = unsafe { ShowWindow(self.hwnd, SW_SHOWNA) };
        } else {
            self.hide();
        }
    }
}

/// 每格文字居中；格与格之间一条上下留 `inset` 的细线。
fn paint_cells(
    hdc: HDC,
    client: RECT,
    cells: &[CellSpec],
    sizes: &[SIZE],
    widths: &[i32],
    separator: COLORREF,
    inset: i32,
) {
    let mut x = 0;
    for (index, ((cell, size), width)) in cells.iter().zip(sizes).zip(widths).enumerate() {
        if index > 0 {
            view::fill_rect(
                hdc,
                RECT {
                    left: x,
                    top: inset,
                    right: x + 1,
                    bottom: client.bottom - inset,
                },
                separator,
            );
        }
        let ox = x + (width - size.cx) / 2;
        let oy = (client.bottom - size.cy) / 2;
        view::draw_text(hdc, cell.font, cell.color, ox, oy, &cell.text);
        x += width;
    }
}

impl Drop for StatusBar {
    fn drop(&mut self) {
        PLACEMENTS.with(|map| map.borrow_mut().remove(&(self.hwnd.0 as isize)));
        let _ = unsafe { DestroyWindow(self.hwnd) };
    }
}

/// 首次出现的位置：主显示器工作区右下角，留出边距与阴影。
fn default_anchor(content: (i32, i32), margin: i32) -> (i32, i32) {
    let work = monitor::primary_work_area();
    let gap = ((EDGE_GAP * margin) / 16).max(EDGE_GAP);
    (
        work.right - margin - gap - content.0,
        work.bottom - margin - gap - content.1,
    )
}

/// 把内容左上角夹进所在显示器的工作区，使整块内容可见。
fn clamp_anchor(anchor: (i32, i32), content: (i32, i32), margin: i32) -> (i32, i32) {
    let work = monitor::work_area_near(POINT {
        x: anchor.0,
        y: anchor.1,
    });
    let x = anchor.0.clamp(
        work.left + margin,
        (work.right - margin - content.0).max(work.left + margin),
    );
    let y = anchor.1.clamp(
        work.top + margin,
        (work.bottom - margin - content.1).max(work.top + margin),
    );
    (x, y)
}

fn placement_of(hwnd: HWND) -> Option<Rc<Placement>> {
    // clone 出来放开借用，再调回调。
    PLACEMENTS.with(|map| map.borrow().get(&(hwnd.0 as isize)).cloned())
}

/// 按下：拖动交给系统移动循环，没拖就是点击；点击不激活；拖动结束报位置。
unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_NCHITTEST => LRESULT(HTCLIENT as isize),
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_LBUTTONDOWN => {
            let mut point = POINT::default();
            let _ = unsafe { GetCursorPos(&mut point) };
            if unsafe { DragDetect(hwnd, point) }.as_bool() {
                let _ = unsafe { ReleaseCapture() };
                unsafe {
                    SendMessageW(
                        hwnd,
                        WM_NCLBUTTONDOWN,
                        Some(WPARAM(HTCAPTION as usize)),
                        Some(LPARAM(0)),
                    )
                };
            } else if let Some(placement) = placement_of(hwnd) {
                // lparam 低 16 位是客户区 x（有符号）。
                placement.on_click((lparam.0 & 0xFFFF) as i16 as i32);
            }
            LRESULT(0)
        }
        WM_EXITSIZEMOVE => {
            if let Some(placement) = placement_of(hwnd) {
                placement.on_moved();
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
