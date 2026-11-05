//! 符号面板窗口：与候选窗同款的不抢焦点、置顶分层窗口，贴在光标下方（从菜单打开时在鼠标旁），尺寸固定。
//! 面板由青简渲染器按主题画成位图贴上去（[`qingjian_render::Renderer::render_panel`]），点击区域随位图一起给出。
//! 鼠标：单击选格、双击上屏、点胶囊 / 标签换组换类、滚轮滚行、点 ✕ 收起，认出来经回调交给 Router；
//! 翻译框的原文点哪里光标放到哪里，按住拖动选字（按下时抓住鼠标，拖出窗口也跟着），Shift 点击扩选区。
//! 双击要窗口类带 `CS_DBLCLKS`。按下不激活窗口，焦点留在应用里。

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, ReleaseCapture, SetCapture, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CS_DBLCLKS, CreateWindowExW, DefWindowProcW, DestroyWindow, GetCursorPos, HTCLIENT,
    HTTRANSPARENT, IDC_ARROW, LoadCursorW, MA_NOACTIVATE, SW_HIDE, SW_SHOWNA, ShowWindow,
    WHEEL_DELTA, WM_CAPTURECHANGED, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCHITTEST, WNDCLASSEXW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{PCWSTR, Result, w};

use qingjian_core::GRID_ROWS;
use qingjian_render::{FontLibrary, HitRegion, HitTarget, PanelHit, Renderer, Theme, system_fonts};

use super::candidates::{place, resolve_dark};
use super::layered;
use super::monitor;
use super::painter::SharedPainter;
use super::window_class::WindowClass;
use crate::dispatch::{PanelFrame, PanelMouse};

const CLASS_NAME: PCWSTR = w!("QingjianSymbolPanel");
static CLASS: WindowClass = WindowClass::new();

const HINT: &str = "Tab 换组 · [ ] 换类 · 双击或空格上屏";

const TRANSLATE_HINT: &str = "Tab 换组 · [ ] 换语言 · 回车上屏";

/// 面板上的鼠标操作交给 Router 的回调（UI 线程上调，投给工人线程）。
pub type PanelEvents = Box<dyn Fn(PanelMouse) + Send>;

/// 窗口过程要读的：上一帧的点击区域、内容区左上角的屏幕坐标与宽高、回调、没凑够一行的滚轮量、拖选到的位置。
struct Shared {
    hits: RefCell<Vec<HitRegion>>,

    origin: Cell<(i32, i32)>,

    size: Cell<(i32, i32)>,

    wheel: Cell<i32>,

    /// 正在翻译框里拖选：上一次报给 Router 的光标位置，没动就不再报。
    drag: Cell<Option<usize>>,

    events: PanelEvents,
}

thread_local! {
    static SHARED: RefCell<Option<Rc<Shared>>> = const { RefCell::new(None) };
}

pub(super) struct PanelWindow {
    hwnd: HWND,

    /// 面板用自己的渲染器：与候选窗共用会打断候选窗的过渡动画；系统绘制时也要有。第一次显示时才建，
    /// 建不成为 `Some(None)`，不再试。
    renderer: RefCell<Option<Option<Renderer>>>,

    /// 候选窗那份渲染器与主题；系统绘制时为空，面板用青简绿。
    painter: SharedPainter,

    shared: Rc<Shared>,

    visible: Cell<bool>,
}

impl PanelWindow {
    pub(super) fn new(events: PanelEvents, painter: SharedPainter) -> Result<Self> {
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
        let shared = Rc::new(Shared {
            hits: RefCell::new(Vec::new()),
            origin: Cell::new((0, 0)),
            size: Cell::new((0, 0)),
            wheel: Cell::new(0),
            drag: Cell::new(None),
            events,
        });
        SHARED.with(|slot| *slot.borrow_mut() = Some(shared.clone()));
        Ok(Self {
            hwnd,
            renderer: RefCell::new(None),
            painter,
            shared,
            visible: Cell::new(false),
        })
    }

    /// 画一帧并显示。`anchor` 是光标矩形；没有时第一次摆在鼠标旁，之后留在原处。
    pub(super) fn show(&self, frame: PanelFrame, anchor: Option<RECT>) {
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
        let theme = self.theme().with_dark(resolve_dark(frame.appearance));
        let rendered = {
            let mut renderer = self.renderer.borrow_mut();
            let Some(renderer) = renderer.get_or_insert_with(|| new_renderer(&theme)) else {
                return;
            };
            renderer.render_panel(&content(frame), &theme, dpi.max(96) as f32 / 96.0)
        };
        let rendered = match rendered {
            Ok(rendered) => rendered,
            Err(error) => {
                tracing::warn!(%error, "symbol panel render failed");
                self.hide();
                return;
            }
        };
        let size = (
            rendered.content_width as i32,
            rendered.content_height as i32,
        );
        let origin = match anchor {
            Some(rect) => {
                let overhang = rendered.visible_overhang();
                place(rect, size, (overhang.0 as i32, overhang.1 as i32))
            }
            None => self.shared.origin.get(),
        };
        let position = (
            origin.0 - rendered.content_x as i32,
            origin.1 - rendered.content_y as i32,
        );
        if let Err(error) = layered::present(self.hwnd, &rendered.pixmap, position) {
            tracing::warn!(%error, "symbol panel present failed");
            self.hide();
            return;
        }
        self.shared.origin.set(origin);
        self.shared.size.set(size);
        *self.shared.hits.borrow_mut() = rendered.hits;
        if !self.visible.replace(true) {
            let _ = unsafe { ShowWindow(self.hwnd, SW_SHOWNA) };
        }
    }

    pub(super) fn hide(&self) {
        let _ = unsafe { ShowWindow(self.hwnd, SW_HIDE) };
        self.visible.set(false);
        self.shared.wheel.set(0);
        self.shared.hits.borrow_mut().clear();
        end_drag(&self.shared);
    }

    /// 主题换了：新主题的字体要装进面板自己的渲染器。
    pub(super) fn theme_changed(&self) {
        let theme = self.theme();
        if let Some(Some(renderer)) = self.renderer.borrow_mut().as_mut() {
            renderer.load_theme_fonts(&theme, system_fonts::family_files);
        }
    }

    /// 候选窗用的主题（浅色那一份）；系统绘制时是青简绿。
    fn theme(&self) -> Theme {
        self.painter
            .borrow()
            .as_ref()
            .map_or_else(Theme::light, |painter| painter.theme().clone())
    }
}

impl Drop for PanelWindow {
    fn drop(&mut self) {
        SHARED.with(|slot| *slot.borrow_mut() = None);
        let _ = unsafe { DestroyWindow(self.hwnd) };
    }
}

/// Router 给的一帧换成渲染器的。
fn content(frame: PanelFrame) -> qingjian_render::PanelFrame {
    qingjian_render::PanelFrame {
        groups: frame.groups,
        group: frame.group,
        sections: frame.sections,
        section: frame.section,
        cells: frame.cells,
        columns: frame.columns,
        rows: GRID_ROWS,
        highlight: frame.highlight,
        footer: frame.footer,
        hint: if frame.translate.is_some() {
            TRANSLATE_HINT
        } else {
            HINT
        }
        .to_owned(),
        translate: frame.translate,
    }
}

fn new_renderer(theme: &Theme) -> Option<Renderer> {
    match FontLibrary::system("zh-CN") {
        Ok(library) => {
            let mut renderer = Renderer::new(library);
            renderer.load_theme_fonts(theme, system_fonts::family_files);
            Some(renderer)
        }
        Err(error) => {
            tracing::warn!(%error, "symbol panel font library failed to load, panel disabled");
            None
        }
    }
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

/// 内容区收鼠标（滚轮在空白处也能滚），阴影那一圈穿到下面的应用。
fn hit_test(lparam: LPARAM) -> LRESULT {
    let x = (lparam.0 & 0xFFFF) as i16 as i32;
    let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
    let inside = shared().is_some_and(|shared| {
        let (x, y) = to_content(&shared, x, y);
        let (width, height) = shared.size.get();
        x >= 0.0 && y >= 0.0 && x < width as f32 && y < height as f32
    });
    if inside {
        LRESULT(HTCLIENT as isize)
    } else {
        LRESULT(HTTRANSPARENT as isize)
    }
}

/// 鼠标下面是哪一块。
fn target_under_cursor(shared: &Shared) -> Option<HitTarget> {
    let (x, y) = cursor_in_content(shared);
    shared
        .hits
        .borrow()
        .iter()
        .find(|region| region.contains(x, y))
        .map(|region| region.target)
}

fn click(hwnd: HWND, double: bool) {
    let Some(shared) = shared() else {
        return;
    };
    let hit = match target_under_cursor(&shared) {
        Some(HitTarget::Panel(PanelHit::Cell(offset))) => PanelMouse::Cell { offset, double },
        Some(HitTarget::Panel(PanelHit::Group(group))) => PanelMouse::Group(group),
        Some(HitTarget::Panel(PanelHit::Section(section))) => PanelMouse::Section(section),
        Some(HitTarget::Panel(PanelHit::Candidate(index))) => PanelMouse::Candidate(index),
        Some(HitTarget::Panel(PanelHit::Caret(position))) => {
            // 按下就开始拖选：抓住鼠标，拖出窗口也收得到移动
            shared.drag.set(Some(position));
            unsafe { SetCapture(hwnd) };
            let extend = unsafe { GetKeyState(VK_SHIFT.0 as i32) } < 0;
            PanelMouse::Caret { position, extend }
        }
        Some(HitTarget::Panel(PanelHit::Close)) => PanelMouse::Close,
        _ => return,
    };
    tracing::debug!(?hit, "symbol panel click");
    (shared.events)(hit);
}

/// 按住拖动：选区从按下的地方拉到鼠标下面的字；挪到原文以外的地方时选区不变。
fn drag() {
    let Some(shared) = shared() else {
        return;
    };
    let Some(last) = shared.drag.get() else {
        return;
    };
    let Some(HitTarget::Panel(PanelHit::Caret(position))) = target_under_cursor(&shared) else {
        return;
    };
    if position != last {
        shared.drag.set(Some(position));
        (shared.events)(PanelMouse::Caret {
            position,
            extend: true,
        });
    }
}

fn end_drag(shared: &Shared) {
    if shared.drag.take().is_some() {
        let _ = unsafe { ReleaseCapture() };
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
            click(hwnd, false);
            LRESULT(0)
        }
        WM_LBUTTONDBLCLK => {
            click(hwnd, true);
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            drag();
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            if let Some(shared) = shared() {
                end_drag(&shared);
            }
            LRESULT(0)
        }
        WM_CAPTURECHANGED => {
            if let Some(shared) = shared() {
                shared.drag.set(None);
            }
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            wheel(wparam);
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
