//! 符号面板窗口：与候选窗同款的不抢焦点、置顶分层窗口，贴在光标下方（从菜单打开时在鼠标旁），尺寸固定。
//! 背景与阴影走分层窗口合成器，内容用 Direct2D 画（[`d2d`]），各块位置在 [`layout`]，绘制在 [`draw`]。
//! 鼠标：单击选格、双击上屏、点胶囊 / 标签换组换类、滚轮滚行、点 ✕ 收起，认出来经回调交给 Router；
//! 双击要窗口类带 `CS_DBLCLKS`。按下不激活窗口，焦点留在应用里。

mod d2d;
mod draw;
mod layout;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    CS_DBLCLKS, CreateWindowExW, DefWindowProcW, DestroyWindow, GetCursorPos, HTCLIENT,
    HTTRANSPARENT, IDC_ARROW, LoadCursorW, MA_NOACTIVATE, SW_HIDE, SW_SHOWNA, ShowWindow,
    WHEEL_DELTA, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_MOUSEACTIVATE, WM_MOUSEWHEEL, WM_NCHITTEST,
    WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{PCWSTR, Result, w};

use self::d2d::{Direct2D, accent_color};
use self::draw::Colors;
use self::layout::{Fonts, PanelLayout};
use super::candidates::theme::Theme;
use super::candidates::{place, resolve_dark};
use super::layered::{self, Layered};
use super::monitor;
use super::window_class::WindowClass;
use crate::dispatch::{PanelFrame, PanelMouse};

const CLASS_NAME: PCWSTR = w!("QingjianSymbolPanel");
static CLASS: WindowClass = WindowClass::new();

/// 面板上的鼠标操作交给 Router 的回调（UI 线程上调，投给工人线程）。
pub type PanelEvents = Box<dyn Fn(PanelMouse) + Send>;

/// 窗口过程要读的：上一帧与它的布局、内容左上角的屏幕坐标、回调、没凑够一行的滚轮量。
struct Shared {
    shown: RefCell<Option<(PanelFrame, PanelLayout)>>,

    origin: Cell<(i32, i32)>,

    wheel: Cell<i32>,

    events: PanelEvents,
}

thread_local! {
    static SHARED: RefCell<Option<Rc<Shared>>> = const { RefCell::new(None) };
}

pub(super) struct PanelWindow {
    hwnd: HWND,

    /// 建不起来（系统缺组件）时为 `None`，面板不显示。
    d2d: Option<Direct2D>,

    /// 按 DPI 建的字体，DPI 变了重建。
    fonts: RefCell<Option<Fonts>>,

    /// 候选窗那套主题（颜色、圆角），DPI 或深浅变了重建。
    theme: RefCell<Option<(u32, bool, Rc<Theme>)>>,

    shared: Rc<Shared>,

    visible: Cell<bool>,
}

impl PanelWindow {
    pub(super) fn new(events: PanelEvents) -> Result<Self> {
        CLASS.ensure(|| WNDCLASSEXW {
            style: CS_DBLCLKS,
            lpfnWndProc: Some(wndproc),
            hInstance: super::module_handle(),
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default(),
            lpszClassName: CLASS_NAME,
            ..Default::default()
        })?;
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                CLASS_NAME,
                w!("青简符号面板"),
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
        let d2d = Direct2D::new()
            .inspect_err(|error| tracing::error!(%error, "Direct2D 初始化失败，符号面板将不显示"))
            .ok();
        let shared = Rc::new(Shared {
            shown: RefCell::new(None),
            origin: Cell::new((0, 0)),
            wheel: Cell::new(0),
            events,
        });
        SHARED.with(|slot| *slot.borrow_mut() = Some(shared.clone()));
        Ok(Self {
            hwnd,
            d2d,
            fonts: RefCell::new(None),
            theme: RefCell::new(None),
            shared,
            visible: Cell::new(false),
        })
    }

    /// 画一帧并显示。`anchor` 是光标矩形；没有时第一次摆在鼠标旁，之后留在原处。
    pub(super) fn show(&self, frame: PanelFrame, anchor: Option<RECT>) {
        let Some(d2d) = &self.d2d else {
            return;
        };
        let anchor = match anchor {
            Some(rect) => Some(rect),
            None if self.visible.get() => None,
            None => Some(cursor_rect()),
        };
        let point = match anchor {
            Some(rect) => POINT {
                x: rect.left,
                y: rect.top,
            },
            None => {
                let (x, y) = self.shared.origin.get();
                POINT { x, y }
            }
        };
        let dpi = monitor::dpi_near(point)
            .or_else(|| Some(unsafe { GetDpiForWindow(self.hwnd) }).filter(|&dpi| dpi != 0))
            .unwrap_or(96);
        let theme = self.theme(dpi, resolve_dark(frame.appearance));
        if self.fonts.borrow().as_ref().is_none_or(|f| f.dpi != dpi) {
            match Fonts::new(d2d, dpi) {
                Ok(fonts) => *self.fonts.borrow_mut() = Some(fonts),
                Err(error) => {
                    tracing::warn!(%error, "符号面板建字体失败");
                    return;
                }
            }
        }
        let fonts = self.fonts.borrow();
        let Some(fonts) = fonts.as_ref() else {
            return;
        };
        let layout = PanelLayout::new(d2d, fonts, &frame);
        let content = layout.size;
        let margin = layered::shadow_margin(dpi);
        let origin = match anchor {
            Some(rect) => place(rect, content, (0, margin)),
            None => self.shared.origin.get(),
        };
        let colors = Colors {
            text: theme.text_color,
            gloss: theme.gloss_color,
            index: theme.index_color,
            separator: theme.pos_color,
            highlight: theme.highlight,
            accent: accent_color(),
            close: mix(theme.background, theme.text_color, 0.1),
        };
        let inset = theme.corner_radius;
        let composed = layered::composite(
            self.hwnd,
            &Layered {
                content,
                margin,
                win_pos: (origin.0 - margin, origin.1 - margin),
                win_size: (content.0 + margin * 2, content.1 + margin * 2),
                background: theme.background,
                corner_radius: theme.corner_radius,
                // 圆角那一圈留给合成器，D2D 只画里面（内容都在内边距以内）
                paint: &|hdc, client| {
                    let area = RECT {
                        left: inset,
                        top: inset,
                        right: client.right - inset,
                        bottom: client.bottom - inset,
                    };
                    let painted = d2d.paint(hdc, area, theme.background, |canvas| {
                        draw::paint(canvas, &frame, &layout, fonts, &colors);
                    });
                    if let Err(error) = painted {
                        tracing::warn!(%error, "符号面板绘制失败");
                    }
                },
            },
        );
        if let Err(error) = composed {
            tracing::warn!(%error, "符号面板贴图失败");
            self.hide();
            return;
        }
        self.shared.origin.set(origin);
        *self.shared.shown.borrow_mut() = Some((frame, layout));
        if !self.visible.replace(true) {
            let _ = unsafe { ShowWindow(self.hwnd, SW_SHOWNA) };
        }
    }

    pub(super) fn hide(&self) {
        let _ = unsafe { ShowWindow(self.hwnd, SW_HIDE) };
        self.visible.set(false);
        self.shared.wheel.set(0);
        *self.shared.shown.borrow_mut() = None;
    }

    fn theme(&self, dpi: u32, dark: bool) -> Rc<Theme> {
        let mut cached = self.theme.borrow_mut();
        match cached.as_ref() {
            Some((d, k, theme)) if *d == dpi && *k == dark => theme.clone(),
            _ => {
                let theme = Rc::new(Theme::new(dpi, dark));
                *cached = Some((dpi, dark, theme.clone()));
                theme
            }
        }
    }
}

impl Drop for PanelWindow {
    fn drop(&mut self) {
        SHARED.with(|slot| *slot.borrow_mut() = None);
        let _ = unsafe { DestroyWindow(self.hwnd) };
    }
}

/// `a` 往 `b` 混 `t`（0–1）。
fn mix(a: COLORREF, b: COLORREF, t: f32) -> COLORREF {
    let channel = |shift: u32| {
        let (x, y) = (
            ((a.0 >> shift) & 0xFF) as f32,
            ((b.0 >> shift) & 0xFF) as f32,
        );
        ((x + (y - x) * t).round() as u32) << shift
    };
    COLORREF(channel(0) | channel(8) | channel(16))
}

/// 鼠标所在处当一行光标，面板摆在它下方。
fn cursor_rect() -> RECT {
    let mut point = POINT::default();
    let _ = unsafe { GetCursorPos(&mut point) };
    RECT {
        left: point.x,
        top: point.y,
        right: point.x,
        bottom: point.y + 16,
    }
}

fn shared() -> Option<Rc<Shared>> {
    SHARED.with(|slot| slot.borrow().clone())
}

/// 屏幕坐标换成内容坐标。
fn to_content(shared: &Shared, x: i32, y: i32) -> (f32, f32) {
    let (ox, oy) = shared.origin.get();
    ((x - ox) as f32, (y - oy) as f32)
}

fn cursor_in_content(shared: &Shared) -> (f32, f32) {
    let mut point = POINT::default();
    let _ = unsafe { GetCursorPos(&mut point) };
    to_content(shared, point.x, point.y)
}

/// 内容区收鼠标，阴影那一圈穿到下面的应用。
fn hit_test(lparam: LPARAM) -> LRESULT {
    let x = (lparam.0 & 0xFFFF) as i16 as i32;
    let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
    let inside = shared().is_some_and(|shared| {
        let (x, y) = to_content(&shared, x, y);
        shared
            .shown
            .borrow()
            .as_ref()
            .is_some_and(|(_, layout)| layout.inside(x, y))
    });
    if inside {
        LRESULT(HTCLIENT as isize)
    } else {
        LRESULT(HTTRANSPARENT as isize)
    }
}

fn click(double: bool) {
    let Some(shared) = shared() else {
        return;
    };
    let (x, y) = cursor_in_content(&shared);
    let hit = shared
        .shown
        .borrow()
        .as_ref()
        .and_then(|(frame, layout)| layout.hit(frame, x, y, double));
    if let Some(hit) = hit {
        tracing::debug!(?hit, "点击符号面板");
        (shared.events)(hit);
    }
}

/// 滚轮：凑够一格（120）滚一行，触控板的小步累加起来；往上滚是负行数。
fn wheel(wparam: WPARAM) {
    let Some(shared) = shared() else {
        return;
    };
    let delta = ((wparam.0 >> 16) & 0xFFFF) as i16 as i32;
    let total = shared.wheel.get() + delta;
    let step = WHEEL_DELTA as i32;
    let rows = total / step;
    shared.wheel.set(total - rows * step);
    if rows != 0 {
        (shared.events)(PanelMouse::Scroll(-rows as isize));
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_NCHITTEST => hit_test(lparam),
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_LBUTTONDOWN => {
            click(false);
            LRESULT(0)
        }
        WM_LBUTTONDBLCLK => {
            click(true);
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            wheel(wparam);
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
