//! 候选窗口：不抢焦点、置顶的分层窗口，跟随光标，画拼音行与候选列表，四周柔和阴影。
//! 缺省交给青简渲染器出位图再贴（[`super::painter`]），配置 `renderer = "system"` 时走 GDI：绘制在 [`view`]，
//! 配色 / 字体在 [`theme`]。绘制内容在 [`RenderData`]，一行的展示形态在 [`row`]。设计语言对齐 macOS 端。

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
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetCursorPos, IDC_ARROW, LoadCursorW, SW_HIDE,
    SW_SHOWNA, ShowWindow, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{Error, PCWSTR, Result, w};

use qingjian_platform::ThemeMode;
use qingjian_platform::protocol::Frame;

pub(crate) use self::render_data::RenderData;
use self::theme::Theme;
use super::layered::{self, Layered};
use super::monitor;
use super::painter::SharedPainter;
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

    /// 上次记进日志的缩放值（窗口 DPI、光标所在显示器 DPI）：变了才再记一条（#146）。
    logged_dpi: Cell<Option<(u32, Option<u32>)>>,

    /// 青简渲染器；`None` 走 GDI。
    painter: SharedPainter,
}

impl CandidateWindow {
    /// 建一个隐藏的候选窗口。
    pub(crate) fn new(painter: SharedPainter) -> Result<Self> {
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
            logged_dpi: Cell::new(None),
            painter,
        })
    }

    /// 刷新内容（不定位、不显示）。
    pub(crate) fn set_content(&self, frame: &Frame) {
        self.data.borrow_mut().set(frame);
    }

    /// 按光标矩形定位并显示：贴光标下方（放不下放上方），四周留出阴影。
    /// 锚点先过 resolve_anchor：全屏/游戏模式或坐标越界时用真实光标兜底（#360）。
    pub(crate) fn show(&self, anchor: RECT) {
        let anchor = resolve_anchor(anchor);
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
                let (content_x, content_y) = place(anchor, content);
                layered::present(
                    self.hwnd,
                    &rendered.pixmap,
                    (
                        content_x - rendered.content_x as i32,
                        content_y - rendered.content_y as i32,
                    ),
                )
            }
            None => self.show_gdi(anchor),
        };
        if updated.is_ok() {
            let _ = unsafe { ShowWindow(self.hwnd, SW_SHOWNA) };
        } else {
            self.hide();
        }
    }

    /// GDI 画法：量尺寸、定位、合成。
    fn show_gdi(&self, anchor: RECT) -> Result<()> {
        let anchor = resolve_anchor(anchor);
        let margin = layered::shadow_margin(self.dpi.get());
        let content = self.preferred_size();
        if content.0 <= 0 || content.1 <= 0 {
            return Err(Error::from(E_INVALIDARG));
        }
        let (content_x, content_y) = place(anchor, content);
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
        let _ = unsafe { ShowWindow(self.hwnd, SW_HIDE) };
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
        let dark = resolve_dark(self.data.borrow().theme_mode);
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

/// 解析定位锚点：普通窗口信任 TSF 上报的组句矩形；全屏/游戏模式或锚点明显越界时，
/// TSF 的坐标经常滞后或停在主屏，改用 GetCursorPos() 的真实光标位置兜底（#360）。
fn resolve_anchor(anchor: RECT) -> RECT {
    if super::status::is_fullscreen() || !anchor_plausible(anchor) {
        let mut pt = POINT::default();
        unsafe { let _ = GetCursorPos(&mut pt); };
        return RECT {
            left: pt.x,
            top: pt.y,
            right: pt.x,
            bottom: pt.y,
        };
    }
    anchor
}

/// 锚点左上角是否落在任一显示器工作区内。落在范围外说明 TSF 坐标不可信。
fn anchor_plausible(anchor: RECT) -> bool {
    let caret = POINT {
        x: anchor.left,
        y: anchor.top,
    };
    let work = monitor::work_area_near(caret);
    anchor.left >= work.left
        && anchor.left < work.right
        && anchor.top >= work.top
        && anchor.top < work.bottom
}

/// 候选窗相对锚点的摆放：内容左上角，贴光标下方，放不下放上方，再放不下贴屏幕内；
/// 都夹在锚点所在显示器工作区里。逻辑与布局无关，供单测。
fn place(anchor: RECT, content: (i32, i32)) -> (i32, i32) {
    let work = monitor::work_area_near(POINT {
        x: anchor.left,
        y: anchor.top,
    });
    place_xy(anchor, content, work)
}

/// 纯摆放计算：work 是锚点所在显示器的工作区。贴光标下方，放不下放上方，再放不下贴屏幕内。
fn place_xy(anchor: RECT, content: (i32, i32), work: RECT) -> (i32, i32) {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 双屏（副屏在右，主屏在左）：光标在副屏，候选窗应出现在副屏工作区内。
    #[test]
    fn place_on_secondary_monitor() {
        // 主屏 0..1920，副屏 1920..3840，任务栏占底部 40px。
        let work = RECT {
            left: 1920,
            top: 0,
            right: 3840,
            bottom: 1040,
        };
        let anchor = RECT {
            left: 2500,
            top: 500,
            right: 2510,
            bottom: 510,
        };
        let (x, y) = place_xy(anchor, (200, 60), work);
        assert_eq!(x, 2500, "x 应贴光标左边缘");
        assert_eq!(y, 512, "y 应贴光标下方 + 间隙");
    }

    /// 光标在屏幕底部，下方放不下 → 翻到光标上方。
    #[test]
    fn place_above_when_no_room_below() {
        let work = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040,
        };
        let anchor = RECT {
            left: 800,
            top: 1000,
            right: 810,
            bottom: 1010,
        };
        let (_, y) = place_xy(anchor, (200, 80), work);
        assert_eq!(y, 918, "应翻到光标上方：1000 - 2 - 80");
    }

    /// 内容比显示器还高：夹在屏幕底部，不越界。
    #[test]
    fn place_clamps_oversized_content() {
        let work = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040,
        };
        let anchor = RECT {
            left: 100,
            top: 100,
            right: 110,
            bottom: 110,
        };
        let (x, y) = place_xy(anchor, (3000, 3000), work);
        // 内容宽 3000 超出工作区 1920：x 夹到工作区左缘（0），不能超过右边界
        assert_eq!(x, 0, "超宽内容贴工作区左缘");
        assert_eq!(y, 0, "超高内容贴工作区顶");
    }

    /// 光标贴右边缘，内容超出 → 向左收，不越过显示器右边界。
    #[test]
    fn place_clamps_right_edge() {
        let work = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040,
        };
        let anchor = RECT {
            left: 1900,
            top: 100,
            right: 1910,
            bottom: 110,
        };
        let (x, _) = place_xy(anchor, (300, 80), work);
        assert_eq!(x, 1620, "x = 1920 - 300，贴右边界");
    }

    /// 普通窗口的锚点落在工作区内 → 原样保留（不被兜底替换）。
    #[test]
    fn anchor_in_work_area_is_kept() {
        let work = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040,
        };
        let anchor = RECT {
            left: 500,
            top: 300,
            right: 510,
            bottom: 310,
        };
        // anchor_plausible 走 monitor::work_area_near，Windows 上依赖真实屏幕；
        // 这里只验证 place_xy 对工作区内锚点的摆放不受影响。
        let (x, y) = place_xy(anchor, (200, 60), work);
        assert_eq!(x, 500);
        assert_eq!(y, 312);
    }
}
