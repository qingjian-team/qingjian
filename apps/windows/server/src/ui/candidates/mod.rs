//! 候选窗口：不抢焦点、置顶的分层窗口，跟随光标，画拼音行与候选列表，四周柔和阴影。
//! 缺省交给青简渲染器出位图再贴（[`super::painter`]），配置 `renderer = "system"` 时走 GDI：绘制在 [`view`]，
//! 配色 / 字体在 [`theme`]。绘制内容在 [`RenderData`]，一行的展示形态在 [`row`]。设计语言对齐 macOS 端。

mod click;
mod key_hint;
mod menu;
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
    CreateWindowExW, DefWindowProcW, DestroyWindow, HWND_TOPMOST, IDC_ARROW, KillTimer,
    LoadCursorW, MSG, SW_HIDE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SetTimer,
    SetWindowPos, ShowWindow, WM_LBUTTONDOWN, WM_MOUSEACTIVATE, WM_NCHITTEST, WM_RBUTTONDOWN,
    WM_RBUTTONUP, WM_TIMER, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{Error, PCWSTR, Result, w};

use qingjian_platform::Appearance;
use qingjian_platform::protocol::Frame;
use qingjian_render::Mode;

use crate::dispatch::{CandidateMenu, KeyHintView};

pub use self::click::CandidateClicks;
use self::key_hint::KeyHintWindow;
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

    /// 旁边的键位提示窗口；建不成为 `None`，不显示键位提示。
    key_hint_window: Option<KeyHintWindow>,

    /// 这一帧要贴的键位提示，`None` 收起。
    key_hint: RefCell<Option<KeyHintView>>,
}

impl CandidateWindow {
    /// 建一个隐藏的候选窗口；点中候选或译词时调 `on_click`。
    pub(crate) fn new(painter: SharedPainter, on_click: CandidateClicks) -> Result<Self> {
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
        if let Err(error) = menu::install() {
            tracing::error!(%error, "建右键菜单窗口失败，右键候选不出菜单");
        }
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
        let key_hint_window = KeyHintWindow::new()
            .inspect_err(|error| tracing::error!(%error, "建键位提示窗口失败，将不显示键位提示"))
            .ok();
        Ok(Self {
            hwnd,
            data,
            dpi: Cell::new(dpi),
            dark: Cell::new(dark),
            logged_dpi: Cell::new(None),
            painter,
            position: Cell::new((0, 0)),
            key_hint_window,
            key_hint: RefCell::new(None),
        })
    }

    /// 刷新内容（不定位、不显示）。
    pub(crate) fn set_content(&self, frame: &Frame, mode: Mode, key_hint: Option<KeyHintView>) {
        self.data.borrow_mut().set(frame, mode);
        *self.key_hint.borrow_mut() = key_hint;
    }

    /// 换了主题：键位提示的渲染器也装上新主题的字体。
    pub(crate) fn theme_changed(&self) {
        if let (Some(window), Some(painter)) =
            (&self.key_hint_window, self.painter.borrow().as_ref())
        {
            window.set_theme(painter.theme());
        }
    }

    /// 按光标矩形定位并显示：贴光标下方（放不下放上方），四周留出阴影。
    pub(crate) fn show(&self, anchor: RECT) {
        menu::close();
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
                let overhang = (
                    rendered.content_y as i32,
                    rendered.pixmap.height() as i32 - rendered.content_y as i32 - content.1,
                );
                let (content_x, content_y) = place(anchor, content, overhang);
                let position = (
                    content_x - rendered.content_x as i32,
                    content_y - rendered.content_y as i32,
                );
                self.position.set(position);
                self.schedule(rendered.next_frame);
                click::set_hits(rendered.hits.clone(), (content_x, content_y));
                layered::present(self.hwnd, &rendered.pixmap, position).map(|()| {
                    Some(RECT {
                        left: content_x,
                        top: content_y,
                        right: content_x + content.0,
                        bottom: content_y + content.1,
                    })
                })
            }
            None => {
                click::set_hits(Vec::new(), (0, 0));
                self.show_gdi(anchor).map(|()| None)
            }
        };
        match updated {
            Ok(content) => {
                // 搜索面板可能在候选窗创建后再弹出，虽然两者都是 topmost，后出现的窗口仍会排在上面。
                // 每次更新候选时重新插到 topmost 队首，保持候选可见且不激活候选窗。
                if let Err(error) = unsafe {
                    SetWindowPos(
                        self.hwnd,
                        Some(HWND_TOPMOST),
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
                    )
                } {
                    tracing::warn!(%error, "候选窗口置顶失败");
                }
                self.show_key_hint(content, anchor);
            }
            Err(_) => self.hide(),
        }
    }

    /// 候选窗画好之后在它旁边贴键位提示；没有键位提示、或走 GDI 画法（`content` 为 `None`）时收起。
    fn show_key_hint(&self, content: Option<RECT>, anchor: RECT) {
        let Some(window) = &self.key_hint_window else {
            return;
        };
        let key_hint = self.key_hint.borrow();
        let painter = self.painter.borrow();
        match (key_hint.as_ref(), content, painter.as_ref()) {
            (Some(view), Some(content), Some(painter)) => window.show(
                view,
                painter.theme(),
                self.dark.get(),
                self.dpi.get(),
                content,
                anchor,
            ),
            _ => window.hide(),
        }
    }

    /// GDI 画法：量尺寸、定位、合成。
    fn show_gdi(&self, anchor: RECT) -> Result<()> {
        let margin = layered::shadow_margin(self.dpi.get());
        let content = self.preferred_size();
        if content.0 <= 0 || content.1 <= 0 {
            return Err(Error::from(E_INVALIDARG));
        }
        // GDI 画法的投影顶影为 0，只有底边那一圈要避
        let (content_x, content_y) = place(anchor, content, (0, margin));
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

    /// 在鼠标处弹右键菜单（深浅与 DPI 跟候选窗）。
    pub(crate) fn show_menu(&self, menu: CandidateMenu) {
        menu::open(menu, self.dark.get(), self.dpi.get());
    }

    pub(crate) fn close_menu(&self) {
        menu::close();
    }

    pub(crate) fn hide(&self) {
        menu::close();
        let _ = unsafe { ShowWindow(self.hwnd, SW_HIDE) };
        if let Some(window) = &self.key_hint_window {
            window.hide();
        }
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
}

impl Drop for CandidateWindow {
    fn drop(&mut self) {
        let _ = unsafe { DestroyWindow(self.hwnd) };
    }
}

/// 内容左上角：贴光标下方，放不下放上方，再放不下贴屏幕内；都夹在所在显示器工作区里。
/// `overhang` 是位图在内容区上、下伸出去的像素（投影、装饰），按主题而定：
/// 贴下方让开上沿那截，贴上方让开下沿那截，投影才不会罩在输入行刚打的字上（微信聊天框实测）。
pub(super) fn place(anchor: RECT, content: (i32, i32), overhang: (i32, i32)) -> (i32, i32) {
    let work = monitor::work_area_near(POINT {
        x: anchor.left,
        y: anchor.top,
    });
    let x = anchor
        .left
        .clamp(work.left, (work.right - content.0).max(work.left));
    let below = anchor.bottom + CARET_GAP + overhang.0.max(0);
    let above = anchor.top - CARET_GAP - overhang.1.max(0) - content.1;
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
        // 按下吞掉（不让默认处理生出 WM_CONTEXTMENU），抬起时弹菜单
        WM_RBUTTONDOWN => LRESULT(0),
        WM_RBUTTONUP => {
            click::right_button_up();
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
