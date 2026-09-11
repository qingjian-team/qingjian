//! 候选窗口：不抢焦点、置顶的浮动弹窗，跟随光标，画组句拼音行与候选列表，四周带柔和阴影。
//!
//! 用**分层窗口**（`WS_EX_LAYERED` + `UpdateLayeredWindow`）而不是普通 `WM_PAINT` 窗口：内容与阴影一起
//! 合成进一张预乘 alpha 的位图，一次贴上去，于是能像 macOS 那样四边柔和淡出、候选框「浮」在应用之上
//! （普通 `CS_DROPSHADOW` 只有右 / 下一道硬边）。合成细节在 [`surface`]，内容绘制在 [`view`]，一行的展示
//! 形态在 [`row`]，配色 / 字体在 [`theme`]。定位靠调用方传进来的光标屏幕矩形（TSF 的 `GetTextExt` 拿）。
//!
//! 设计语言对齐 macOS 端（层级 / 配色角色 / 特色排布），平台底层用 Windows 的（系统 UI 字体、GDI 绘制 +
//! 分层窗口阴影）。整句补全 / 云朵图标下一步补。

pub(crate) mod row;
pub(crate) mod surface;
pub(crate) mod theme;
pub(crate) mod view;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Once;

use windows::Win32::Foundation::{
    ERROR_CLASS_ALREADY_EXISTS, GetLastError, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM,
};
use windows::Win32::Graphics::Gdi::{
    GetDC, GetMonitorInfoW, HBRUSH, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
    ReleaseDC,
};
use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
use windows::Win32::UI::HiDpi::{GetDpiForSystem, GetDpiForWindow};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, IDC_ARROW, LoadCursorW, RegisterClassExW,
    SW_HIDE, SW_SHOWNA, ShowWindow, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{Error, Result, w};

use qingjian_platform::protocol::{Frame, PreeditKind};
use qingjian_platform::{LayoutMode, ThemeMode};

use self::row::Row;
use self::theme::Theme;

/// 窗口类名（进程内注册一次）。
const CLASS_NAME: windows::core::PCWSTR = w!("QingjianCandidateWindow");

/// 光标行与候选窗之间的间隙（逻辑像素基准，按 DPI 缩放）。
const CARET_GAP: i32 = 2;

/// 阴影留白（逻辑像素，按 DPI 缩放）：窗口在内容四周各留这么宽画柔和阴影（够定向主阴影向下偏移后淡出）。
const SHADOW_MARGIN: i32 = 16;

/// 阴影留白按 DPI 缩放后的像素数（至少 1）。
fn shadow_margin(dpi: u32) -> i32 {
    ((SHADOW_MARGIN * dpi as i32) / 96).max(1)
}

/// 把外观模式解析成实际深浅：`Light`/`Dark` 直接定，`System` 读系统主题。
fn resolve_dark(mode: ThemeMode) -> bool {
    match mode {
        ThemeMode::Light => false,
        ThemeMode::Dark => true,
        ThemeMode::System => system_prefers_dark(),
    }
}

/// 系统当前是否深色：读 `HKCU\...\Themes\Personalize\AppsUseLightTheme`（DWORD，0=深色）。读不到当浅色。
fn system_prefers_dark() -> bool {
    let subkey = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
    let value = w!("AppsUseLightTheme");
    let mut data: u32 = 1;
    let mut size = core::mem::size_of::<u32>() as u32;
    // SAFETY: 传入的缓冲与长度匹配；键 / 值名是静态宽字符串。
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            subkey,
            value,
            RRF_RT_REG_DWORD,
            None,
            Some((&mut data as *mut u32).cast()),
            Some(&mut size),
        )
    };
    status.0 == 0 && data == 0
}

/// 候选窗口一次绘制要用的全部内容。
pub(crate) struct RenderData {
    /// 配色与字体（随 DPI 重建）。
    pub(super) theme: Rc<Theme>,

    /// 顶部拼音行的各段（文本 + 种类）。
    pub(super) preedit: Vec<(String, PreeditKind)>,

    /// 光标在拼音行里的字符位置。
    pub(super) cursor: usize,

    /// 候选行。
    pub(super) rows: Vec<Row>,

    /// 高亮行下标（页内，从 0 起）。
    pub(super) highlight: usize,

    /// 右下角页码，只有多页时有。
    pub(super) footer: Option<String>,

    /// 候选排布（竖排 / 横排），由 Server 随帧下发。
    pub(super) layout: LayoutMode,

    /// 外观模式（跟随系统 / 浅色 / 深色），随帧下发；`System` 由窗口按系统主题解析成实际深浅。
    theme_mode: ThemeMode,
}

impl RenderData {
    fn empty(theme: Rc<Theme>) -> Self {
        Self {
            theme,
            preedit: Vec::new(),
            cursor: 0,
            rows: Vec::new(),
            highlight: usize::MAX,
            footer: None,
            layout: LayoutMode::default(),
            theme_mode: ThemeMode::default(),
        }
    }

    /// 用一帧刷新内容（不触发绘制；由 [`CandidateWindow::show`] 定位后合成上屏）。
    fn set(&mut self, frame: &Frame) {
        self.layout = frame.layout;
        self.theme_mode = frame.theme;
        self.preedit = frame
            .preedit
            .iter()
            .map(|segment| (segment.text.clone(), segment.kind))
            .collect();
        self.cursor = frame.cursor;
        self.rows = frame
            .candidates
            .items
            .iter()
            .enumerate()
            .map(|(i, candidate)| Row::from_candidate(i, candidate))
            .collect();
        self.highlight = frame.highlight;
        self.footer =
            (frame.page_count > 1).then(|| format!("{}/{}", frame.page + 1, frame.page_count));
    }
}

/// 候选窗口。分层窗口的内容由 [`surface::update`] 一次贴上，窗口过程只用默认处理，故 `data` 直接内嵌
/// 不必交裸指针给窗口过程；`Drop` 里销毁窗口。
pub(crate) struct CandidateWindow {
    /// 窗口句柄。
    hwnd: HWND,

    /// 绘制内容。
    data: RefCell<RenderData>,

    /// 上次用的 DPI；变了就重建主题字体。
    dpi: Cell<u32>,

    /// 上次解析出的深浅（`ThemeMode` + 系统主题共同决定）；变了就重建主题配色。
    dark: Cell<bool>,
}

impl CandidateWindow {
    /// 建一个隐藏的候选窗口。失败（类注册 / 建窗口出错）返回 `Err`，调用方降级为无候选 UI。
    pub(crate) fn new() -> Result<Self> {
        ensure_class_registered()?;
        // SAFETY: 无参数系统调用。
        let dpi = unsafe { GetDpiForSystem() }.max(96);
        // 起手按 System 解析（首帧到来前只是隐藏窗口）。
        let dark = resolve_dark(ThemeMode::default());
        let data = RefCell::new(RenderData::empty(Rc::new(Theme::new(dpi, dark))));
        // SAFETY: 类已注册；分层窗口内容后续由 UpdateLayeredWindow 设。
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
                Some(super::dll_instance()),
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

    /// 按光标屏幕矩形 `anchor` 定位并显示。内容贴光标下方（放不下放上方），四周留出阴影。
    pub(crate) fn show(&self, anchor: RECT) {
        self.sync_theme();
        let margin = shadow_margin(self.dpi.get());
        let content = self.preferred_size();
        if content.0 <= 0 || content.1 <= 0 {
            self.hide();
            return;
        }
        // place 定内容左上角（内容留在工作区内）；窗口再往外扩 margin 给阴影（越屏那点阴影可裁掉）。
        let (content_x, content_y) = place(anchor, content);
        let win_pos = (content_x - margin, content_y - margin);
        let win_size = (content.0 + margin * 2, content.1 + margin * 2);
        let updated = {
            let data = self.data.borrow();
            surface::update(self.hwnd, &data, content, margin, win_pos, win_size)
        };
        if updated.is_ok() {
            // SAFETY: hwnd 有效；SW_SHOWNA 显示但不激活（绝不抢焦点）。
            let _ = unsafe { ShowWindow(self.hwnd, SW_SHOWNA) };
        } else {
            self.hide();
        }
    }

    /// 收起窗口。
    pub(crate) fn hide(&self) {
        // SAFETY: hwnd 有效。
        let _ = unsafe { ShowWindow(self.hwnd, SW_HIDE) };
    }

    /// DPI 或深浅变了就重建主题（字体随 DPI、配色随深浅）。深浅由当前帧的 `ThemeMode` 解析（`System`
    /// 读系统主题）。在每次 `show` 前调一次，配置切换或系统换主题后下一次弹窗即生效。
    fn sync_theme(&self) {
        // SAFETY: hwnd 有效。
        let dpi = unsafe { GetDpiForWindow(self.hwnd) };
        let dpi = if dpi == 0 { self.dpi.get() } else { dpi };
        let dark = resolve_dark(self.data.borrow().theme_mode);
        if dpi != self.dpi.get() || dark != self.dark.get() {
            self.data.borrow_mut().theme = Rc::new(Theme::new(dpi, dark));
            self.dpi.set(dpi);
            self.dark.set(dark);
        }
    }

    /// 量内容需要的大小（借一个窗口 DC 测字）。不含阴影留白。
    fn preferred_size(&self) -> (i32, i32) {
        // SAFETY: hwnd 有效；DC 用完即还。
        unsafe {
            let hdc = GetDC(Some(self.hwnd));
            let size = view::preferred_size(hdc, &self.data.borrow());
            ReleaseDC(Some(self.hwnd), hdc);
            (size.cx, size.cy)
        }
    }
}

impl Drop for CandidateWindow {
    fn drop(&mut self) {
        // SAFETY: hwnd 由本对象建、只在这里销毁。
        let _ = unsafe { DestroyWindow(self.hwnd) };
    }
}

/// 内容左上角坐标：贴光标下方；放不下放上方；再放不下贴屏幕内。坐标都在光标所在显示器的工作区里夹取。
/// `content` 是**内容**尺寸（不含阴影留白），保证内容留在工作区内。
fn place(anchor: RECT, content: (i32, i32)) -> (i32, i32) {
    let work = work_area(anchor);
    let gap = CARET_GAP;
    let x = anchor
        .left
        .clamp(work.left, (work.right - content.0).max(work.left));
    let below = anchor.bottom + gap;
    let above = anchor.top - gap - content.1;
    let y = if below + content.1 <= work.bottom {
        below
    } else if above >= work.top {
        above
    } else {
        (work.bottom - content.1).max(work.top)
    };
    (x, y)
}

/// 光标所在（最近）显示器的工作区矩形。
fn work_area(anchor: RECT) -> RECT {
    let point = POINT {
        x: anchor.left,
        y: anchor.top,
    };
    let mut info = MONITORINFO {
        cbSize: core::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: info.cbSize 已填；MonitorFromPoint 始终回一个显示器句柄。
    unsafe {
        let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
        if GetMonitorInfoW(monitor, &mut info).as_bool() {
            info.rcWork
        } else {
            // 拿不到就给个大范围，至少别把窗口夹没。
            RECT {
                left: 0,
                top: 0,
                right: i32::MAX,
                bottom: i32::MAX,
            }
        }
    }
}

/// 进程内注册一次窗口类。是否失败缓存进原子布尔，重复调用直接读。
fn ensure_class_registered() -> Result<()> {
    use std::sync::atomic::{AtomicBool, Ordering};
    static REGISTER: Once = Once::new();
    static FAILED: AtomicBool = AtomicBool::new(false);
    REGISTER.call_once(|| {
        // SAFETY: 光标 / 类结构都是标准用法；失败按 GetLastError 判断。
        let outcome = unsafe {
            let cursor = LoadCursorW(None, IDC_ARROW).unwrap_or_default();
            let class = WNDCLASSEXW {
                cbSize: core::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(wndproc),
                hInstance: super::dll_instance(),
                hCursor: cursor,
                hbrBackground: HBRUSH::default(),
                lpszClassName: CLASS_NAME,
                ..Default::default()
            };
            RegisterClassExW(&class)
        };
        if outcome == 0 {
            // SAFETY: 紧跟失败的调用读取线程 last error。
            let last = unsafe { GetLastError() };
            // 类已存在不算失败（DLL 卸载后重载、同进程多个 TIP 实例都可能撞上）。
            if last != ERROR_CLASS_ALREADY_EXISTS {
                FAILED.store(true, Ordering::SeqCst);
            }
        }
    });
    if FAILED.load(Ordering::SeqCst) {
        Err(Error::from(windows::Win32::Foundation::E_FAIL))
    } else {
        Ok(())
    }
}

/// 窗口过程：分层窗口内容由 `UpdateLayeredWindow` 设，无需 `WM_PAINT`，全交默认处理。
unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: 转交系统默认窗口过程。
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}
