//! 候选右键菜单的窗口：候选窗上面一块不抢焦点的分层窗口，菜单由青简渲染器画成位图贴上去（与 macOS 同一份画法）。
//! 左上角贴着鼠标，下面 / 右边放不下就往上 / 往左翻；鼠标移动换悬停项重画，点中一项就关窗、经点击回调交回 Router。
//! 敲键、候选窗重画或收起时关掉。窗口与状态都在 UI 线程上（只有一个菜单）。

use std::cell::RefCell;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetCursorPos, HTCLIENT, HTTRANSPARENT, IDC_ARROW, LoadCursorW,
    MA_NOACTIVATE, SW_HIDE, SW_SHOWNA, ShowWindow, WM_LBUTTONDOWN, WM_MOUSEACTIVATE, WM_MOUSEMOVE,
    WM_NCHITTEST, WM_RBUTTONDOWN, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{PCWSTR, Result, w};

use qingjian_render::{FontLibrary, HitTarget, Rendered, Renderer};

use super::click::{self, CandidateMouse};
use crate::dispatch::CandidateMenu;
use crate::ui::window_class::WindowClass;
use crate::ui::{layered, monitor};

const CLASS_NAME: PCWSTR = w!("QingjianCandidateMenu");

/// `TrackMouseEvent` 登记后鼠标离开窗口时来的消息（定义在 `Win32_UI_Controls`，为一个常量不开那个特性）。
const WM_MOUSELEAVE: u32 = 0x02A3;
static CLASS: WindowClass = WindowClass::new();

struct Menu {
    hwnd: HWND,

    /// 菜单用自己的渲染器：与候选窗共用会打断候选窗的过渡动画。第一次弹时才建；建不成为 `Some(None)`。
    renderer: Option<Option<Renderer>>,

    /// 弹着的那份；`None` 是收着。
    open: Option<CandidateMenu>,

    hovered: Option<usize>,

    dark: bool,

    dpi: u32,

    /// 位图左上角的屏幕坐标。
    position: (i32, i32),

    rendered: Option<Rendered>,
}

thread_local! {
    static MENU: RefCell<Option<Menu>> = const { RefCell::new(None) };
}

/// 建窗口（隐藏）。候选窗建好时调。
pub(super) fn install() -> Result<()> {
    CLASS.ensure(|| WNDCLASSEXW {
        lpfnWndProc: Some(wndproc),
        hInstance: super::super::module_handle(),
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default(),
        lpszClassName: CLASS_NAME,
        ..Default::default()
    })?;
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
            CLASS_NAME,
            w!("青简候选菜单"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(super::super::module_handle()),
            None,
        )?
    };
    MENU.with(|menu| {
        *menu.borrow_mut() = Some(Menu {
            hwnd,
            renderer: None,
            open: None,
            hovered: None,
            dark: false,
            dpi: 96,
            position: (0, 0),
            rendered: None,
        });
    });
    Ok(())
}

/// 在 `menu.point` 弹出菜单。
pub(super) fn open(menu: CandidateMenu, dark: bool, dpi: u32) {
    let point = POINT {
        x: menu.point.0,
        y: menu.point.1,
    };
    let shown = MENU.with(|cell| {
        let mut cell = cell.borrow_mut();
        let state = cell.as_mut()?;
        state.open = Some(menu);
        state.hovered = None;
        state.dark = dark;
        state.dpi = dpi;
        let rendered = state.render()?;
        let (width, height) = (
            rendered.content_width as i32,
            rendered.content_height as i32,
        );
        let work = monitor::work_area_near(point);
        let x = if point.x + width > work.right {
            point.x - width
        } else {
            point.x
        };
        let y = if point.y + height > work.bottom {
            point.y - height
        } else {
            point.y
        };
        state.position = (x - rendered.content_x as i32, y - rendered.content_y as i32);
        Some(state.hwnd)
    });
    // 显示放在借用之外：ShowWindow 会同步发 WM_NCHITTEST 之类回到窗口过程
    if let Some(hwnd) = shown
        && present()
    {
        let _ = unsafe { ShowWindow(hwnd, SW_SHOWNA) };
    }
}

/// 收起菜单；没弹着就什么都不做。
pub(super) fn close() {
    let hwnd = MENU.with(|cell| {
        let mut cell = cell.borrow_mut();
        let state = cell.as_mut()?;
        state.open.take()?;
        state.hovered = None;
        Some(state.hwnd)
    });
    if let Some(hwnd) = hwnd {
        let _ = unsafe { ShowWindow(hwnd, SW_HIDE) };
    }
}

impl Menu {
    /// 按当前悬停项画一遍，留着结果给命中与贴图用。
    fn render(&mut self) -> Option<&Rendered> {
        let titles: Vec<&str> = self.open.as_ref()?.items.iter().map(|(_, t)| *t).collect();
        let renderer = self
            .renderer
            .get_or_insert_with(|| match FontLibrary::system("zh-CN") {
                Ok(library) => Some(Renderer::new(library)),
                Err(error) => {
                    tracing::warn!(%error, "右键菜单的字体库加载失败");
                    None
                }
            })
            .as_mut()?;
        let scale = self.dpi.max(96) as f32 / 96.0;
        match renderer.render_menu(&titles, self.hovered, self.dark, scale) {
            Ok(rendered) => self.rendered = Some(rendered),
            Err(error) => {
                tracing::warn!(%error, "右键菜单渲染失败");
                return None;
            }
        }
        self.rendered.as_ref()
    }

    /// 屏幕上这一点落在第几项上。
    fn item_at(&self, x: i32, y: i32) -> Option<usize> {
        let rendered = self.rendered.as_ref()?;
        let x = (x - self.position.0 - rendered.content_x as i32) as f32;
        let y = (y - self.position.1 - rendered.content_y as i32) as f32;
        rendered
            .hits
            .iter()
            .find(|region| region.contains(x, y))
            .and_then(|region| match region.target {
                HitTarget::MenuItem(item) => Some(item),
                HitTarget::Candidate(_) | HitTarget::Translation { .. } => None,
            })
    }
}

/// 把最近画的位图贴上去；成功返回 `true`。
fn present() -> bool {
    MENU.with(|cell| {
        let cell = cell.borrow();
        let Some(state) = cell.as_ref() else {
            return false;
        };
        let Some(rendered) = &state.rendered else {
            return false;
        };
        layered::present(state.hwnd, &rendered.pixmap, state.position).is_ok()
    })
}

fn cursor() -> Option<(i32, i32)> {
    let mut point = POINT::default();
    unsafe { GetCursorPos(&mut point) }.ok()?;
    Some((point.x, point.y))
}

/// 光标在第几项上；状态正被借着（窗口过程被同步调进来）时当没有。
fn item_under_cursor() -> Option<usize> {
    let (x, y) = cursor()?;
    MENU.with(|cell| cell.try_borrow().ok()?.as_ref()?.item_at(x, y))
}

/// 悬停项变了就重画。
fn hover(hwnd: HWND, item: Option<usize>) {
    let changed = MENU.with(|cell| {
        let Ok(mut cell) = cell.try_borrow_mut() else {
            return false;
        };
        let Some(state) = cell.as_mut() else {
            return false;
        };
        if state.open.is_none() || state.hovered == item {
            return false;
        }
        state.hovered = item;
        state.render().is_some()
    });
    if changed {
        present();
    }
    if item.is_some() {
        let mut track = TRACKMOUSEEVENT {
            cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
            dwFlags: TME_LEAVE,
            hwndTrack: hwnd,
            dwHoverTime: 0,
        };
        let _ = unsafe { TrackMouseEvent(&mut track) };
    }
}

/// 点中一项：先关菜单，再把它的编号连同那一格交回 Router。
fn pick() {
    let Some(item) = item_under_cursor() else {
        return;
    };
    let picked = MENU.with(|cell| {
        let cell = cell.borrow();
        let menu = cell.as_ref()?.open.as_ref()?;
        Some((menu.offset, menu.items.get(item)?.0))
    });
    close();
    if let Some((offset, id)) = picked {
        click::send(CandidateMouse::MenuPick { offset, id });
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        // 投影那圈穿透，只有卡片本身收鼠标
        WM_NCHITTEST => {
            let x = (lparam.0 & 0xFFFF) as i16 as i32;
            let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
            let inside = MENU.with(|cell| {
                cell.try_borrow().ok().and_then(|cell| {
                    let state = cell.as_ref()?;
                    let rendered = state.rendered.as_ref()?;
                    let left = state.position.0 + rendered.content_x as i32;
                    let top = state.position.1 + rendered.content_y as i32;
                    Some(
                        x >= left
                            && x < left + rendered.content_width as i32
                            && y >= top
                            && y < top + rendered.content_height as i32,
                    )
                })
            });
            LRESULT(if inside == Some(true) {
                HTCLIENT as isize
            } else {
                HTTRANSPARENT as isize
            })
        }
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_MOUSEMOVE => {
            hover(hwnd, item_under_cursor());
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            hover(hwnd, None);
            LRESULT(0)
        }
        WM_LBUTTONDOWN | WM_RBUTTONDOWN => {
            pick();
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
