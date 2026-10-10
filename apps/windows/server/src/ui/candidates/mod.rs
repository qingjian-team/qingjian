//! 候选窗口：不抢焦点、置顶的分层窗口，跟随光标，画拼音行与候选列表，四周柔和阴影。
//! 缺省交给青简渲染器出位图再贴（[`super::painter`]），配置 `renderer = "system"` 时走 GDI：绘制在 [`view`]，
//! 配色 / 字体在 [`theme`]。绘制内容在 [`RenderData`]，一行的展示形态在 [`row`]。设计语言对齐 macOS 端。

mod click;
mod paging;
mod render_data;
pub(crate) mod row;
pub(crate) mod theme;
pub(crate) mod view;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use windows::Win32::Foundation::{E_INVALIDARG, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC};
use windows::Win32::UI::HiDpi::{GetDpiForSystem, GetDpiForWindow};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, IDC_ARROW, KillTimer, LoadCursorW, MSG,
    SW_HIDE, SW_SHOWNA, SetTimer, ShowWindow, WM_LBUTTONDOWN, WM_MOUSEACTIVATE, WM_NCHITTEST,
    WM_TIMER, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_POPUP,
};
use windows::core::{Error, PCWSTR, Result, w};

use qingjian_platform::Appearance;
use qingjian_platform::protocol::Frame;
use qingjian_render::Mode;

pub use self::click::CandidateClicks;
pub use self::paging::PageClicks;
use self::paging::PagingWindow;
pub(crate) use self::render_data::RenderData;
use self::theme::Theme;
use super::layered::{self, Layered};
use super::monitor;
use super::painter::SharedPainter;
use super::window_class::WindowClass;

const CLASS_NAME: PCWSTR = w!("QingjianCandidateWindow");
static CLASS: WindowClass = WindowClass::new();

/// 动画定时器的 id（间隔按渲染器给的 `next_frame`；系统定时器精度约 15.6 ms）。
const ANIMATION_TIMER: usize = 1;

/// 光标行与候选窗之间的间隙（逻辑像素）。
const CARET_GAP: i32 = 2;

/// 按外观模式解析深浅；`System` 读系统主题。
pub(super) fn resolve_dark(mode: Appearance) -> bool {
    match mode {
        Appearance::Light => false,
        Appearance::Dark => true,
        Appearance::System => system_prefers_dark(),
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

    paging: PagingWindow,

    /// 绘制内容。
    data: RefCell<RenderData>,

    /// 上次用的 DPI，变了重建字体。
    dpi: Cell<u32>,

    /// 上次解析出的深浅，变了重建配色。
    dark: Cell<bool>,

    /// 上次记进日志的缩放值（窗口 DPI、光标所在显示器 DPI）：变了才再记一条（#146）。
    logged_dpi: Cell<Option<(u32, Option<u32>)>>,

    /// 青简渲染器；`None` 走 GDI。
    painter: SharedPainter,

    /// 最近一次贴图时窗口左上角的屏幕坐标；动画帧贴在同一处。
    position: Cell<(i32, i32)>,
}

impl CandidateWindow {
    /// 建一个隐藏的候选窗口；点中候选或译词时调 `on_click`。
    pub(crate) fn new(
        painter: SharedPainter,
        on_click: CandidateClicks,
        on_page: PageClicks,
    ) -> Result<Self> {
        CLASS.ensure(|| WNDCLASSEXW {
            lpfnWndProc: Some(wndproc),
            hInstance: super::module_handle(),
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default(),
            lpszClassName: CLASS_NAME,
            ..Default::default()
        })?;
        let dpi = unsafe { GetDpiForSystem() }.max(96);
        let dark = resolve_dark(Appearance::default());
        let data = RefCell::new(RenderData::empty(Rc::new(Theme::new(dpi, dark))));
        // NOACTIVATE：显示时不抢应用焦点。鼠标只在候选与译词上收，其余穿透（见 [`click`]）。
        click::install(on_click);
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
        let paging = match PagingWindow::new(hwnd, on_page) {
            Ok(paging) => paging,
            Err(error) => {
                let _ = unsafe { DestroyWindow(hwnd) };
                return Err(error);
            }
        };
        Ok(Self {
            hwnd,
            paging,
            data,
            dpi: Cell::new(dpi),
            dark: Cell::new(dark),
            logged_dpi: Cell::new(None),
            painter,
            position: Cell::new((0, 0)),
        })
    }

    /// 刷新内容（不定位、不显示）。
    pub(crate) fn set_content(&self, frame: &Frame, mode: Mode) {
        self.data.borrow_mut().set(frame, mode);
    }

    /// 按光标矩形定位并显示：贴光标下方（放不下放上方），四周留出阴影。
    pub(crate) fn show(&self, anchor: RECT) {
        self.sync_theme(anchor);
        let rendered = {
            let data = self.data.borrow();
            self.painter.borrow_mut().as_mut().and_then(|painter| {
                painter.render_frame(
                    &data.render_frame(),
                    data.layout,
                    self.dark.get(),
                    self.dpi.get(),
                )
            })
        };
        let updated = match rendered {
            Some(rendered) => {
                let content = (
                    rendered.content_width as i32,
                    rendered.content_height as i32,
                );
                if content.0 <= 0 || content.1 <= 0 {
                    self.hide();
                    return;
                }
                let (content_x, content_y) = self.place_with_paging(anchor, content);
                let position = (
                    content_x - rendered.content_x as i32,
                    content_y - rendered.content_y as i32,
                );
                self.position.set(position);
                self.schedule(rendered.next_frame);
                click::set_hits(rendered.hits.clone(), (content_x, content_y));
                layered::present(self.hwnd, &rendered.pixmap, position)
            }
            None => {
                click::set_hits(Vec::new(), (0, 0));
                self.show_gdi(anchor)
            }
        };
        if updated.is_ok() {
            let _ = unsafe { ShowWindow(self.hwnd, SW_SHOWNA) };
        } else {
            self.hide();
        }
    }

    /// GDI 画法：量尺寸、定位、合成。
    fn show_gdi(&self, anchor: RECT) -> Result<()> {
        let margin = layered::shadow_margin(self.dpi.get());
        let content = self.preferred_size();
        if content.0 <= 0 || content.1 <= 0 {
            return Err(Error::from(E_INVALIDARG));
        }
        let (content_x, content_y) = self.place_with_paging(anchor, content);
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
    }

    pub(crate) fn hide(&self) {
        self.paging.hide();
        let _ = unsafe { ShowWindow(self.hwnd, SW_HIDE) };
        click::set_hits(Vec::new(), (0, 0));
        self.stop_animation();
        if let Some(painter) = self.painter.borrow_mut().as_mut() {
            painter.forget();
        }
    }

    /// 这条消息是候选窗的动画定时器。
    pub(crate) fn is_animation_timer(&self, msg: &MSG) -> bool {
        msg.message == WM_TIMER && msg.hwnd == self.hwnd && msg.wParam.0 == ANIMATION_TIMER
    }

    /// 定时器每跳：要动画的下一帧贴在原处；播完就停。
    pub(crate) fn animation_frame(&self) {
        let rendered = self
            .painter
            .borrow_mut()
            .as_mut()
            .and_then(|painter| painter.tick());
        let Some(rendered) = rendered else {
            self.stop_animation();
            return;
        };
        let _ = layered::present(self.hwnd, &rendered.pixmap, self.position.get());
        self.schedule(rendered.next_frame);
    }

    /// 按渲染器给的间隔（过渡 16 ms、循环动画 33 ms）起或改定时器；`None` 停。同一个 id 再 SetTimer 就是改间隔。
    fn schedule(&self, next_frame: Option<std::time::Duration>) {
        match next_frame {
            Some(interval) => {
                let ms = interval.as_millis().clamp(10, 1000) as u32;
                // SAFETY: 本线程建的窗口；定时器消息由 UI 线程的消息循环截下交给 animation_frame
                unsafe { SetTimer(Some(self.hwnd), ANIMATION_TIMER, ms, None) };
            }
            None => self.stop_animation(),
        }
    }

    fn stop_animation(&self) {
        // SAFETY: 没在跳时 KillTimer 返回错误，忽略
        let _ = unsafe { KillTimer(Some(self.hwnd), ANIMATION_TIMER) };
    }

    /// DPI 或深浅变了就重建主题；每次 `show` 前调。
    ///
    /// DPI 取光标所在显示器的：窗口藏着时改了缩放（或睡眠唤醒后多显示器重排），
    /// `GetDpiForWindow` 会停在旧值，候选字就大小不对（#146）。
    fn sync_theme(&self, anchor: RECT) {
        let caret = POINT {
            x: anchor.left,
            y: anchor.top,
        };
        let monitor_dpi = monitor::dpi_near(caret);
        let window_dpi = unsafe { GetDpiForWindow(self.hwnd) };
        let dpi = match (monitor_dpi, window_dpi) {
            (Some(dpi), _) => dpi,
            (None, 0) => self.dpi.get(),
            (None, dpi) => dpi,
        };
        self.log_dpi(caret, window_dpi, monitor_dpi, dpi);
        let dark = resolve_dark(self.data.borrow().appearance);
        if dpi != self.dpi.get() || dark != self.dark.get() {
            self.data.borrow_mut().theme = Rc::new(Theme::new(dpi, dark));
            self.dpi.set(dpi);
            self.dark.set(dark);
        }
    }

    /// 缩放值变了就记一条，多显示器 / 睡眠唤醒的问题从日志里能看出取到的是哪个值（#146）。
    fn log_dpi(&self, caret: POINT, window_dpi: u32, monitor_dpi: Option<u32>, used: u32) {
        if self.logged_dpi.replace(Some((window_dpi, monitor_dpi)))
            == Some((window_dpi, monitor_dpi))
        {
            return;
        }
        tracing::info!(
            window_dpi,
            ?monitor_dpi,
            used,
            system_dpi = unsafe { GetDpiForSystem() },
            caret_x = caret.x,
            caret_y = caret.y,
            "候选窗口缩放值"
        );
    }

    /// 内容需要的大小（不含阴影留白）。
    fn preferred_size(&self) -> (i32, i32) {
        let hdc = unsafe { GetDC(Some(self.hwnd)) };
        let size = view::preferred_size(hdc, &self.data.borrow());
        unsafe { ReleaseDC(Some(self.hwnd), hdc) };
        (size.cx, size.cy)
    }

    fn place_with_paging(&self, anchor: RECT, content: (i32, i32)) -> (i32, i32) {
        let data = self.data.borrow();
        let pager = self.paging.size(&data.theme, data.page, data.page_count);
        let gap = if pager.1 > 0 {
            data.theme.row_padding
        } else {
            0
        };
        let position = place(anchor, (content.0.max(pager.0), content.1 + gap + pager.1));
        self.paging.show(
            (
                position.0 + content.0.max(pager.0) - pager.0,
                position.1 + content.1 + gap,
            ),
            &data.theme,
            self.dpi.get(),
            data.page,
            data.page_count,
        );
        position
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

/// 分层窗口无需 `WM_PAINT`；鼠标交给 [`click`]，其余全交默认处理。
unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_NCHITTEST => click::hit_test(lparam),
        WM_MOUSEACTIVATE => click::mouse_activate(),
        WM_LBUTTONDOWN => {
            click::button_down();
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
