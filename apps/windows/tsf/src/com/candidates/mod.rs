//! 系统界面宿主里的候选窗口：在 TSF DLL 进程内自绘。
//!
//! 开始屏幕、任务栏搜索、商店应用这些宿主的 z-band 比普通顶层窗高，Server 进程的分层窗盖不过去
//! （除非 Server exe 有 uiAccess + 签名 + 装进 Program Files）。候选项窗建在 **DLL 进程里**、
//! owner 设成宿主窗口，就跟着宿主一起合成，不受这条限制；小狼毫（Weasel）与竹简输入法都是这个做法。
//!
//! 画法与 Server 自己的候选窗完全一致：同一份协议帧经 [`CandidateView`] 换算、同一个 [`Renderer`]
//! 出位图，DLL 只把位图贴到分层窗口上。主题、字号来自 Server 下发的 [`InputSettings`]；TSF 进程
//! 里读不到用户主题目录（AppContainer 宿主尤其读不到），所以这里用内置缺省主题——形状、字号、
//! 深浅都对，只是不套用户的皮肤。
//!
//! 这一层只管「窗口」：命中区的点击回 Server（[`CandidateHit`]），选词仍走 Server 那条链路。

mod layered;
mod monitor;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::Mutex;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
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
use qingjian_render::{
    CandidateView, FontLibrary, HitRegion, HitTarget, Layout, Mode, Rendered, Renderer, TextSizes,
    Theme,
};

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

/// 候选窗外观里由 Server 下发的那几项（字号 / 字重 / 倒排）。其余（主题、字族）在 DLL 里用缺省。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Style {
    /// 候选字号（点）；0 为用主题的。
    pub candidate: f32,

    /// 译文字号（点）；0 为用主题的。
    pub annotation: f32,

    /// 候选字字重；`None` 用主题的。
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

/// 候选窗此刻的点击状态：命中区（内容区像素）、内容区左上角的屏幕坐标、回 Server 的会话客户端。
struct Clicks {
    hits: Vec<HitRegion>,
    origin: (i32, i32),
    client: SharedClient,
}

thread_local! {
    /// 本线程只有一扇进程内候选窗；窗口过程从这里取点击区，不碰窗口对象本身。
    static CLICKS: RefCell<Option<Clicks>> = const { RefCell::new(None) };
}

/// 一次绘制用的渲染器与主题：DPI、深浅、字号变了就重建。
struct Paint {
    renderer: Renderer,
    theme: Theme,
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
    view: RefCell<CandidateView>,

    /// 渲染器与主题；建窗后第一次显示时才建（字体库加载不便宜）。
    paint: RefCell<Option<Paint>>,

    /// Server 下发的画法设置。
    style: Cell<Style>,

    /// 上一次画的是不是倒着排。
    reversed: Cell<bool>,

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
            view: RefCell::new(CandidateView::default()),
            paint: RefCell::new(None),
            style: Cell::new(Style::default()),
            reversed: Cell::new(false),
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
    pub(crate) fn show(&self, frame: &Frame, mode: Mode, anchor: RECT) {
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
        self.view.borrow_mut().set(frame, mode);

        let Some(mut rendered) = self.render(self.reversed.get()) else {
            self.hide();
            return;
        };
        let content = (
            rendered.content_width as i32,
            rendered.content_height as i32,
        );
        if content.0 <= 0 || content.1 <= 0 {
            self.hide();
            return;
        }
        let overhang = rendered.visible_overhang();
        let overhang = (overhang.0 as i32, overhang.1 as i32);
        let (_, y) = place(anchor, content, overhang);

        // 「竖排在光标上方时倒着排」：摆到上方才倒，倒排会改高亮顺序，要重画一次。
        let vertical = self.view.borrow().layout == LayoutMode::Vertical;
        let want_reversed = self.style.get().reverse_above && vertical && y < anchor.top;
        if want_reversed != self.reversed.get()
            && let Some(redrawn) = self.render(want_reversed)
        {
            self.reversed.set(want_reversed);
            rendered = redrawn;
        }
        let content = (
            rendered.content_width as i32,
            rendered.content_height as i32,
        );
        let overhang = rendered.visible_overhang();
        let (x, y) = place(anchor, content, (overhang.0 as i32, overhang.1 as i32));
        let position = (x - rendered.content_x as i32, y - rendered.content_y as i32);
        if let Err(error) = layered::present(self.hwnd, &rendered.pixmap, position) {
            log(&format!("tsf candidate present failed: {error}"));
            self.hide();
            return;
        }
        CLICKS.with(|clicks| {
            if let Some(clicks) = clicks.borrow_mut().as_mut() {
                clicks.hits = rendered.hits.clone();
                clicks.origin = (x, y);
            }
        });
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_SHOWNA);
        }
        self.visible.set(true);
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
        self.reversed.set(false);
        CLICKS.with(|clicks| {
            if let Some(clicks) = clicks.borrow_mut().as_mut() {
                clicks.hits.clear();
                clicks.origin = (0, 0);
            }
        });
        if let Some(paint) = self.paint.borrow_mut().as_mut() {
            paint.renderer.forget();
        }
    }

    /// 用当前展示数据画一帧；渲染器还没建好或画失败为 `None`。
    fn render(&self, reversed: bool) -> Option<Rendered> {
        let view = self.view.borrow();
        let layout = match view.layout {
            LayoutMode::Vertical => Layout::Vertical,
            LayoutMode::Horizontal => Layout::Horizontal,
        };
        let mut frame = view.render_frame();
        frame.reversed = reversed;
        let mut paint = self.paint.borrow_mut();
        let paint = paint.as_mut()?;
        let theme = paint.theme.with_dark(paint.dark);
        let scale = paint.dpi as f32 / 96.0;
        match paint.renderer.render(&frame, layout, &theme, scale) {
            Ok(rendered) => Some(rendered),
            Err(error) => {
                log(&format!("tsf candidate render failed: {error}"));
                None
            }
        }
    }

    /// 按 DPI / 深浅 / 字号建好渲染器；没变就不动。
    fn ensure_paint(&self, dpi: u32, dark: bool) {
        let style = self.style.get();
        if let Some(paint) = self.paint.borrow().as_ref()
            && paint.dpi == dpi
            && paint.dark == dark
            && paint.style == style
        {
            return;
        }
        let library = match FontLibrary::system("zh-CN") {
            Ok(library) => library,
            Err(error) => {
                log(&format!(
                    "tsf candidate font library failed, keeping server drawing: {error}"
                ));
                self.hide();
                return;
            }
        };
        let mut renderer = Renderer::new(library);
        renderer.set_material_support(false);
        let theme = Theme::light()
            .with_text_sizes(TextSizes::new(style.candidate, style.annotation))
            .with_candidate_weight(style.weight)
            .with_user_font(false)
            // 动画会让第一帧停在过渡开头（整窗接近透明），系统界面里不能这么闪。
            .with_animations(false);
        renderer.load_theme_fonts(&theme, qingjian_render::system_fonts::family_files);
        *self.paint.borrow_mut() = Some(Paint {
            renderer,
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
fn place(anchor: RECT, content: (i32, i32), overhang: (i32, i32)) -> (i32, i32) {
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

/// `WM_NCHITTEST` 给的屏幕坐标（有符号 16 位打包）。
fn lparam_point(lparam: LPARAM) -> POINT {
    POINT {
        x: (lparam.0 & 0xFFFF) as i16 as i32,
        y: ((lparam.0 >> 16) & 0xFFFF) as i16 as i32,
    }
}

/// 屏幕这一点落在哪个目标上。
fn target_at(point: POINT) -> Option<HitTarget> {
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
                    HitTarget::Candidate(offset) => send_hit(CandidateHit::Candidate(offset)),
                    HitTarget::Translation { candidate, sense } => {
                        send_hit(CandidateHit::Translation { candidate, sense });
                    }
                    HitTarget::Page(step) => send_hit(CandidateHit::Page(step)),
                    HitTarget::MenuItem(_) | HitTarget::Panel(_) => {}
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
