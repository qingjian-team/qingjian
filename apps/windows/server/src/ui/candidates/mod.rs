//! 候选窗口：不抢焦点、置顶的浮动弹窗，跟随光标，画拼音行与候选列表，四周带柔和阴影。
//!
//! 用分层窗口（`WS_EX_LAYERED` + `UpdateLayeredWindow`）：内容与阴影合成进一张预乘 alpha 的位图一次贴上，
//! 于是能像 macOS 那样四边柔和淡出（普通 `CS_DROPSHADOW` 只有右 / 下一道硬边）。合成在 [`surface`]，
//! 内容绘制在 [`view`]，一行的展示形态在 [`row`]，配色 / 字体在 [`theme`]。
//! 设计语言对齐 macOS 端，平台底层用 Windows 的（系统 UI 字体、GDI）。

pub(crate) mod row;
pub(crate) mod surface;
pub(crate) mod theme;
pub(crate) mod view;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    GetDC, GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint, ReleaseDC,
};
use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
use windows::Win32::UI::HiDpi::{GetDpiForSystem, GetDpiForWindow};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, IDC_ARROW, LoadCursorW, SW_HIDE, SW_SHOWNA,
    ShowWindow, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_POPUP,
};
use windows::core::{PCWSTR, Result, w};

use qingjian_platform::protocol::{Frame, PreeditKind};
use qingjian_platform::{LayoutMode, ThemeMode};

use self::row::Row;
use self::theme::Theme;
use super::window_class::WindowClass;

const CLASS_NAME: PCWSTR = w!("QingjianCandidateWindow");
static CLASS: WindowClass = WindowClass::new();

/// 光标行与候选窗之间的间隙（逻辑像素）。
const CARET_GAP: i32 = 2;

/// 内容四周留给阴影的宽度（逻辑像素，按 DPI 缩放）。
const SHADOW_MARGIN: i32 = 16;

fn shadow_margin(dpi: u32) -> i32 {
    ((SHADOW_MARGIN * dpi as i32) / 96).max(1)
}

fn resolve_dark(mode: ThemeMode) -> bool {
    match mode {
        ThemeMode::Light => false,
        ThemeMode::Dark => true,
        ThemeMode::System => system_prefers_dark(),
    }
}

/// 读 `HKCU\...\Themes\Personalize\AppsUseLightTheme`（0 = 深色）。读不到当浅色。
fn system_prefers_dark() -> bool {
    let subkey = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
    let value = w!("AppsUseLightTheme");
    let mut data: u32 = 1;
    let mut size = core::mem::size_of::<u32>() as u32;
    // SAFETY: 缓冲与长度匹配；键 / 值名是静态宽字符串。
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

/// 一次绘制要用的全部内容。
pub(crate) struct RenderData {
    /// 配色与字体（随 DPI / 深浅重建）。
    pub(super) theme: Rc<Theme>,

    /// 顶部拼音行的各段。
    pub(super) preedit: Vec<(String, PreeditKind)>,

    /// 光标在拼音行里的字符位置。
    pub(super) cursor: usize,

    /// 候选行。
    pub(super) rows: Vec<Row>,

    /// 高亮行下标（页内）。
    pub(super) highlight: usize,

    /// 页码，只有多页时有。
    pub(super) footer: Option<String>,

    /// 整句补全：画在拼音行右侧。
    pub(super) sentence: Option<String>,

    /// 候选排布，随帧下发。
    pub(super) layout: LayoutMode,

    /// 外观模式，随帧下发；`System` 由窗口按系统主题解析。
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
            sentence: None,
            layout: LayoutMode::default(),
            theme_mode: ThemeMode::default(),
        }
    }

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
        self.sentence = frame.sentence.clone();
    }
}

/// 候选窗口。内容由 [`surface::update`] 一次贴上，窗口过程只用默认处理，所以 `data` 不必交给窗口过程。
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
    /// 建一个隐藏的候选窗口。失败返回 `Err`，调用方降级为无候选 UI。
    pub(crate) fn new() -> Result<Self> {
        CLASS.ensure(|| WNDCLASSEXW {
            lpfnWndProc: Some(wndproc),
            hInstance: super::module_handle(),
            // SAFETY: 系统光标。
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default(),
            lpszClassName: CLASS_NAME,
            ..Default::default()
        })?;
        // SAFETY: 无参数系统调用。
        let dpi = unsafe { GetDpiForSystem() }.max(96);
        let dark = resolve_dark(ThemeMode::default());
        let data = RefCell::new(RenderData::empty(Rc::new(Theme::new(dpi, dark))));
        // SAFETY: 类已注册；NOACTIVATE 保证不抢焦点。
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

    /// 按光标矩形定位并显示：内容贴光标下方（放不下放上方），四周留出阴影。
    pub(crate) fn show(&self, anchor: RECT) {
        self.sync_theme();
        let margin = shadow_margin(self.dpi.get());
        let content = self.preferred_size();
        if content.0 <= 0 || content.1 <= 0 {
            self.hide();
            return;
        }
        let (content_x, content_y) = place(anchor, content);
        let win_pos = (content_x - margin, content_y - margin);
        let win_size = (content.0 + margin * 2, content.1 + margin * 2);
        let updated = surface::update(
            self.hwnd,
            &self.data.borrow(),
            content,
            margin,
            win_pos,
            win_size,
        );
        if updated.is_ok() {
            // SAFETY: hwnd 有效；SW_SHOWNA 显示但不激活。
            let _ = unsafe { ShowWindow(self.hwnd, SW_SHOWNA) };
        } else {
            self.hide();
        }
    }

    pub(crate) fn hide(&self) {
        // SAFETY: hwnd 有效。
        let _ = unsafe { ShowWindow(self.hwnd, SW_HIDE) };
    }

    /// DPI 或深浅变了就重建主题。每次 `show` 前调，配置切换 / 系统换主题后下一次弹窗生效。
    fn sync_theme(&self) {
        // SAFETY: hwnd 有效。
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

    /// 内容需要的大小（借一个窗口 DC 测字），不含阴影留白。
    fn preferred_size(&self) -> (i32, i32) {
        // SAFETY: hwnd 有效；DC 用完即还。
        let hdc = unsafe { GetDC(Some(self.hwnd)) };
        let size = view::preferred_size(hdc, &self.data.borrow());
        // SAFETY: 与上面的 GetDC 配对。
        unsafe { ReleaseDC(Some(self.hwnd), hdc) };
        (size.cx, size.cy)
    }
}

impl Drop for CandidateWindow {
    fn drop(&mut self) {
        // SAFETY: hwnd 由本对象建，只在这里销毁。
        let _ = unsafe { DestroyWindow(self.hwnd) };
    }
}

/// 内容左上角坐标：贴光标下方；放不下放上方；再放不下贴屏幕内。都夹在光标所在显示器的工作区里。
fn place(anchor: RECT, content: (i32, i32)) -> (i32, i32) {
    let work = work_area(anchor);
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

/// 光标所在（最近）显示器的工作区。拿不到就给个大范围，至少别把窗口夹没。
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
    let found = unsafe {
        let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
        GetMonitorInfoW(monitor, &mut info).as_bool()
    };
    if found {
        info.rcWork
    } else {
        RECT {
            left: 0,
            top: 0,
            right: i32::MAX,
            bottom: i32::MAX,
        }
    }
}

/// 分层窗口内容由 `UpdateLayeredWindow` 设，无需 `WM_PAINT`，全交默认处理。
unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: 转交系统默认窗口过程。
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}
