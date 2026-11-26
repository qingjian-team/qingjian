//! 系统界面宿主里的候选窗口：在 TSF DLL 进程内自绘。
//!
//! 开始屏幕、任务栏搜索、商店应用这些宿主的 z-band 比普通顶层窗高，Server 进程的分层窗盖不过去
//! （除非 Server exe 有 uiAccess + 签名 + 装进 Program Files）。候选项窗建在 **DLL 进程里**、
//! owner 设成宿主窗口，就跟着宿主一起合成，不受这条限制；小狼毫（Weasel）与竹简输入法都是这个做法。
//!
//! 画法是本 crate 里的轻量 GDI（[`paint`]），不链渲染库：DLL 会被加载进每个应用进程，带上整套渲染库
//! 不划算；TSF 进程里也读不到用户主题目录，皮肤本来就画不出来。外观与 Server 候选窗不必一致，
//! 选词、翻页、高亮、矩阵这些行为一致。字号 / 字重来自 Server 下发的 [`InputSettings`]。
//!
//! 这一层只管「窗口」：命中区的点击回 Server（[`CandidateHit`]），选词仍走 Server 那条链路。

mod monitor;
pub(crate) mod paint;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::Mutex;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC};
use windows::Win32::Security::{GetTokenInformation, TokenAppContainerSid};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetCursorPos, GetWindowThreadProcessId,
    HTCLIENT, HTTRANSPARENT, IDC_ARROW, LoadCursorW, MA_NOACTIVATE, SW_HIDE, SW_SHOWNA, ShowWindow,
    WHEEL_DELTA, WM_LBUTTONUP, WM_MOUSEACTIVATE, WM_MOUSEWHEEL, WM_NCHITTEST, WNDCLASSEXW,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP,
};
use windows::core::{PCWSTR, Result, w};

use qingjian_platform::protocol::{CandidateHit, Frame, InputSettings};
use qingjian_platform::{Appearance, LayoutMode};

use super::dll_instance;
use super::log::log;
use super::service::SharedClient;
use super::window_class::WindowClass;

const CLASS_NAME: PCWSTR = w!("QingjianTsfCandidateWindow");
static CLASS: WindowClass = WindowClass::new();

/// 光标行与候选窗之间的间隙（逻辑像素），与 Server 候选窗一致。
const CARET_GAP: i32 = 2;

/// 系统界面宿主的进程名白名单（小写）：开始屏幕 / 任务栏搜索 / Shell 浮层。
const SHELL_PROCS: &[&str] = &[
    "startmenuexperiencehost.exe",
    "searchexperiencehost.exe",
    "searchhost.exe",
    "shellhost.exe",
    "shellexperiencehost.exe",
    "textinputhost.exe",
    "applicationframehost.exe",
];

/// 候选窗外观里由 Server 下发的那几项（字号 / 字重 / 倒排）。其余（配色、字族）在 DLL 里用内置缺省。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Style {
    /// 候选字号（点）；0 为用内置的。
    pub candidate: f32,

    /// 译文字号（点）；0 为用内置的。
    pub annotation: f32,

    /// 候选字字重；`None` 用常规字重。
    pub weight: Option<u16>,

    /// 竖排窗口在光标上方时倒着排。
    pub reverse_above: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            candidate: 0.0,
            annotation: 0.0,
            weight: None,
            reverse_above: false,
        }
    }
}

impl Style {
    /// 从 Server 下发的按键行为设置里取画法相关的几项。
    pub(crate) fn from_settings(input: &InputSettings) -> Self {
        Self {
            candidate: input.candidate_font_size.get().unwrap_or(0.0),
            annotation: input.annotation_font_size.get().unwrap_or(0.0),
            weight: input.candidate_weight,
            reverse_above: input.reverse_above,
        }
    }
}

/// 宿主是不是「Server 自绘窗会被盖住」的那类进程：AppContainer（商店应用）或沉浸式 Shell 进程。
///
/// 按窗口所在进程判：先看令牌里有没有 AppContainer SID，再看进程名是否命中白名单。
/// 结果按 hwnd 缓存（组句期间每键都会问一次，`OpenProcess` 不必每键都跑）。
pub(crate) fn immersive_host(hwnd: u64) -> bool {
    static CACHE: Mutex<Option<HashMap<u64, bool>>> = Mutex::new(None);
    if hwnd == 0 {
        return false;
    }
    if let Ok(cache) = CACHE.lock()
        && let Some(cache) = cache.as_ref()
        && let Some(&known) = cache.get(&hwnd)
    {
        return known;
    }
    let immersive = classify(hwnd);
    if let Ok(mut cache) = CACHE.lock() {
        cache
            .get_or_insert_with(HashMap::new)
            .insert(hwnd, immersive);
    }
    immersive
}

fn classify(hwnd: u64) -> bool {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(HWND(hwnd as _), Some(&mut pid)) };
    if pid == 0 {
        return false;
    }
    let Ok(process) = (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) })
    else {
        return false;
    };

    // 进程名（小写）：AppContainer 与 Shell 白名单两条都用它记日志。
    let mut buf = [0u16; 512];
    let mut size = buf.len() as u32;
    let named = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut size,
        )
    }
    .is_ok();
    let name = if named {
        String::from_utf16_lossy(&buf[..size as usize])
            .rsplit('\\')
            .next()
            .unwrap_or_default()
            .to_lowercase()
    } else {
        String::new()
    };

    // 1) AppContainer SID：商店应用等 UWP 宿主。
    let mut len = 0u32;
    let _ = unsafe {
        GetTokenInformation(
            process,
            TokenAppContainerSid,
            None::<*mut core::ffi::c_void>,
            0,
            &mut len,
        )
    };
    let _ = unsafe { windows::Win32::Foundation::CloseHandle(process) };
    if len > 0 {
        log(&format!(
            "candidate host classified app={name} kind=appcontainer"
        ));
        return true;
    }

    // 2) 进程名：开始屏幕 / 搜索框等 immersive Shell 进程不是 AppContainer。
    if is_shell_process(&name) {
        log(&format!("candidate host classified app={name} kind=shell"));
        return true;
    }
    false
}

/// 候选窗此刻的点击状态：命中区（内容区像素）、窗口左上角的屏幕坐标、回 Server 的会话客户端。
struct Clicks {
    hits: Vec<paint::HitRegion>,
    origin: (i32, i32),
    client: SharedClient,
}

thread_local! {
    /// 本线程只有一扇进程内候选窗；窗口过程从这里取点击区，不碰窗口对象本身。
    static CLICKS: RefCell<Option<Clicks>> = const { RefCell::new(None) };
}

/// 一次绘制用的主题：DPI、深浅、字号变了就重建（字体是 GDI 资源，重建就要删旧的）。
struct Paint {
    theme: paint::Theme,
    dpi: u32,
    dark: bool,
    style: Style,
}

/// 进程内候选窗口（每个 TSF 线程一扇）。
pub(crate) struct CandidateWindow {
    hwnd: HWND,

    /// 建窗时绑的宿主窗口：换宿主就重建，owner 决定继承哪个 z-band。
    host: HWND,

    /// 当前一帧换算出的展示数据。
    view: RefCell<paint::View>,

    /// 主题；建窗后第一次显示时才建。
    paint: RefCell<Option<Paint>>,

    /// Server 下发的画法设置。
    style: Cell<Style>,

    /// 窗口此刻可见。
    visible: Cell<bool>,
}

impl CandidateWindow {
    /// 建一扇隐藏的候选窗，owner 是宿主窗口（继承宿主 z-band）。
    pub(crate) fn new(host: HWND, client: SharedClient) -> Result<Self> {
        CLASS.ensure(|| WNDCLASSEXW {
            lpfnWndProc: Some(wndproc),
            hInstance: dll_instance(),
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default(),
            lpszClassName: CLASS_NAME,
            ..Default::default()
        })?;
        // 不置顶、不激活：owner 是宿主窗口，z-band 跟着宿主走；`WS_EX_TOOLWINDOW` 不进 Alt+Tab。
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                CLASS_NAME,
                w!("青简候选"),
                WS_POPUP,
                0,
                0,
                0,
                0,
                Some(host),
                None,
                Some(dll_instance()),
                None,
            )?
        };
        CLICKS.with(|clicks| {
            *clicks.borrow_mut() = Some(Clicks {
                hits: Vec::new(),
                origin: (0, 0),
                client,
            })
        });
        Ok(Self {
            hwnd,
            host,
            view: RefCell::new(paint::View::default()),
            paint: RefCell::new(None),
            style: Cell::new(Style::default()),
            visible: Cell::new(false),
        })
    }

    pub(crate) fn host(&self) -> HWND {
        self.host
    }

    pub(crate) fn set_style(&self, style: Style) {
        self.style.set(style);
        // 下次显示按新字号重建字体。
        *self.paint.borrow_mut() = None;
    }

    /// 按光标矩形摆位并显示这一帧；空帧收起。
    pub(crate) fn show(&self, frame: &Frame, anchor: RECT) {
        if frame.is_empty() {
            self.hide();
            return;
        }
        let dpi = monitor::dpi_near(POINT {
            x: anchor.left,
            y: anchor.top,
        })
        .or_else(|| {
            let dpi = unsafe { GetDpiForWindow(self.hwnd) };
            (dpi >= 96).then_some(dpi)
        })
        .unwrap_or(96);
        let dark = resolve_dark(frame.appearance);
        self.ensure_paint(dpi, dark);
        self.view.borrow_mut().set(frame);

        let Some(size) = self.measure() else {
            self.hide();
            return;
        };
        let (x, y) = place(anchor, size);

        // 「竖排在光标上方时倒着排」：第一项贴着光标。倒排不改尺寸，量完再定。
        let vertical = self.view.borrow().layout == LayoutMode::Vertical;
        self.view.borrow_mut().reversed =
            self.style.get().reverse_above && vertical && y < anchor.top;

        let first = !self.visible.get();
        let started = std::time::Instant::now();
        let Some((canvas, hits)) = self.render(size) else {
            self.hide();
            return;
        };
        if let Err(error) = paint::canvas::present(self.hwnd, &canvas, (x, y)) {
            log(&format!("tsf candidate present failed: {error}"));
            self.hide();
            return;
        }
        CLICKS.with(|clicks| {
            if let Some(clicks) = clicks.borrow_mut().as_mut() {
                clicks.hits = hits;
                clicks.origin = (x, y);
            }
        });
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_SHOWNA);
        }
        self.visible.set(true);
        if first {
            log(&format!(
                "tsf candidate first paint dpi={dpi} dark={dark} elapsed_ms={}",
                started.elapsed().as_millis()
            ));
        }
    }

    /// 收起窗口（组句结束 / 换回普通宿主 / 贴图失败）。
    pub(crate) fn hide(&self) {
        if !self.visible.get() {
            return;
        }
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
        self.visible.set(false);
        CLICKS.with(|clicks| {
            if let Some(clicks) = clicks.borrow_mut().as_mut() {
                clicks.hits.clear();
                clicks.origin = (0, 0);
            }
        });
    }

    /// 量出这一帧需要多大；主题还没建好为 `None`。
    fn measure(&self) -> Option<(i32, i32)> {
        let paint = self.paint.borrow();
        let paint = paint.as_ref()?;
        let screen = unsafe { GetDC(None) };
        let size = paint::preferred_size(screen, &paint.theme, &self.view.borrow());
        unsafe {
            let _ = ReleaseDC(None, screen);
        }
        (size.cx > 0 && size.cy > 0).then_some((size.cx, size.cy))
    }

    /// 画一帧：铺底、落笔、补 alpha。返回位图与命中区。
    fn render(&self, size: (i32, i32)) -> Option<(paint::Canvas, Vec<paint::HitRegion>)> {
        let paint = self.paint.borrow();
        let paint = paint.as_ref()?;
        let mut canvas = match paint::Canvas::new(size.0, size.1) {
            Ok(canvas) => canvas,
            Err(error) => {
                log(&format!("tsf candidate canvas failed: {error}"));
                return None;
            }
        };
        canvas.fill_background(
            paint.theme.background,
            paint.theme.border,
            paint.theme.corner_radius,
        );
        let client = RECT {
            left: 0,
            top: 0,
            right: size.0,
            bottom: size.1,
        };
        let hits = paint::paint(canvas.dc(), &paint.theme, &self.view.borrow(), client);
        canvas.finish(paint.theme.corner_radius);
        Some((canvas, hits))
    }

    /// 按 DPI / 深浅 / 字号建好主题；没变就不动。
    fn ensure_paint(&self, dpi: u32, dark: bool) {
        let started = std::time::Instant::now();
        let style = self.style.get();
        if let Some(paint) = self.paint.borrow().as_ref()
            && paint.dpi == dpi
            && paint.dark == dark
            && paint.style == style
        {
            return;
        }
        let scale = dpi as f32 / 96.0;
        // 0 字号是「用内置的」：16 / 12 像素在 96 DPI 下的观感，与 Server 缺省一致。
        let candidate = if style.candidate > 0.0 {
            (style.candidate * dpi as f32 / 72.0).round() as i32
        } else {
            (16.0 * scale).round() as i32
        };
        let annotation = if style.annotation > 0.0 {
            (style.annotation * dpi as f32 / 72.0).round() as i32
        } else {
            (12.0 * scale).round() as i32
        };
        let theme = paint::Theme::new(dpi, dark, candidate.max(8), annotation.max(6), style.weight);
        log(&format!(
            "tsf candidate paint ready dpi={dpi} dark={dark} elapsed_ms={}",
            started.elapsed().as_millis()
        ));
        *self.paint.borrow_mut() = Some(Paint {
            theme,
            dpi,
            dark,
            style,
        });
    }
}

impl Drop for CandidateWindow {
    fn drop(&mut self) {
        CLICKS.with(|clicks| *clicks.borrow_mut() = None);
        let _ = unsafe { DestroyWindow(self.hwnd) };
    }
}

/// 按外观模式解析深浅；`System` 读系统主题。
fn resolve_dark(mode: Appearance) -> bool {
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

/// 候选窗摆在哪：贴光标下方，放不下放上方，左右夹在工作区内。
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

/// `WM_NCHITTEST` 给的屏幕坐标（有符号 16 位打包）。
fn lparam_point(lparam: LPARAM) -> POINT {
    POINT {
        x: (lparam.0 & 0xFFFF) as i16 as i32,
        y: ((lparam.0 >> 16) & 0xFFFF) as i16 as i32,
    }
}

/// 屏幕这一点落在哪个目标上。
fn target_at(point: POINT) -> Option<paint::HitTarget> {
    CLICKS.with(|clicks| {
        let clicks = clicks.borrow();
        let clicks = clicks.as_ref()?;
        let (x, y) = (
            (point.x - clicks.origin.0) as f32,
            (point.y - clicks.origin.1) as f32,
        );
        clicks
            .hits
            .iter()
            .find(|region| region.contains(x, y))
            .map(|region| region.target)
    })
}

fn send_hit(hit: CandidateHit) {
    CLICKS.with(|clicks| {
        let clicks = clicks.borrow();
        let Some(clicks) = clicks.as_ref() else {
            return;
        };
        if let Ok(mut guard) = clicks.client.try_borrow_mut()
            && let Some(client) = guard.as_mut()
            && let Err(error) = client.candidate_hit(hit)
        {
            log(&format!("failed to report candidate hit: {error}"));
        }
    });
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_NCHITTEST => {
            if target_at(lparam_point(lparam)).is_some() {
                LRESULT(HTCLIENT as isize)
            } else {
                LRESULT(HTTRANSPARENT as isize)
            }
        }
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_LBUTTONUP => {
            let mut point = POINT::default();
            if unsafe { GetCursorPos(&mut point) }.is_ok()
                && let Some(target) = target_at(point)
            {
                match target {
                    paint::HitTarget::Candidate(offset) => {
                        send_hit(CandidateHit::Candidate(offset));
                    }
                    paint::HitTarget::Translation { candidate, sense } => {
                        send_hit(CandidateHit::Translation { candidate, sense });
                    }
                }
            }
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            // 系统把滚轮发给光标下的窗口；凑够一格才翻页。
            let delta = ((wparam.0 >> 16) & 0xFFFF) as i16 as i32;
            let steps = delta / WHEEL_DELTA as i32;
            if steps != 0 {
                send_hit(CandidateHit::Page(steps as isize));
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// 进程名（小写）是不是已知的沉浸式 Shell 进程。
fn is_shell_process(name: &str) -> bool {
    SHELL_PROCS.contains(&name)
}

#[cfg(test)]
mod tests;
