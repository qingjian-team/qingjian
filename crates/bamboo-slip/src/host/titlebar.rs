//! Windows 自绘标题栏：保留标准窗口样式（圆角、阴影、边缘缩放与 Aero Snap），通过子类钩子
//! 在 `WM_NCCALCSIZE` 里把顶部非客户区缩到 0，把客户区铺到窗口最顶；在 `WM_NCHITTEST` 里按场景树
//! 标出的 [`WindowRegion`] 给系统回报 `HTCAPTION`、`HTMINBUTTON`、`HTMAXBUTTON`、`HTCLOSE`，
//! 使拖动、双击最大化、右键系统菜单与 Windows 11 最大化按钮的贴靠布局都按原生方式工作。

#[cfg(any(target_os = "windows", test))]
use crate::ui::{WindowHit, WindowRegion};

/// 按画序从最上层往下找包含 `(x, y)` 的元素，返回它声明的标题栏区域；
/// 如果最上层命中的是普通交互控件（搜索框、弹出菜单、遮罩等），返回 `None`（交给客户区处理）。
#[cfg(any(target_os = "windows", test))]
pub(crate) fn hit_region(regions: &[WindowHit], x: f32, y: f32) -> Option<WindowRegion> {
    regions
        .iter()
        .rev()
        .find(|hit| {
            let (left, top, width, height) = hit.rect;
            x >= left && x < left + width && y >= top && y < top + height
        })
        .and_then(|hit| hit.region)
}

#[cfg(target_os = "windows")]
mod win32 {
    use std::ffi::c_void;
    use std::mem::size_of;
    use std::sync::{Arc, Mutex};

    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
    use windows::Win32::Graphics::Dwm::DwmExtendFrameIntoClientArea;
    use windows::Win32::Graphics::Gdi::ScreenToClient;
    use windows::Win32::UI::Controls::MARGINS;
    use windows::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        TME_LEAVE, TME_NONCLIENT, TRACKMOUSEEVENT, TrackMouseEvent,
    };
    use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClientRect, HTCAPTION, HTCLIENT, HTCLOSE, HTMAXBUTTON, HTMINBUTTON, HTTOP, HTTOPLEFT,
        HTTOPRIGHT, IsZoomed, NCCALCSIZE_PARAMS, PostMessageW, SM_CXPADDEDBORDER, SM_CYSIZEFRAME,
        SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
        SWP_NOSIZE, SWP_NOZORDER, SetWindowPos, ShowWindow, WM_CLOSE, WM_NCCALCSIZE, WM_NCDESTROY,
        WM_NCHITTEST, WM_NCLBUTTONDOWN, WM_NCLBUTTONUP, WM_NCMOUSELEAVE, WM_NCMOUSEMOVE,
    };
    use winit::event_loop::EventLoopProxy;
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use winit::window::Window;

    use super::hit_region;
    use crate::backdrop::Backdrop;
    use crate::host::HostEvent;
    use crate::ui::{WindowHit, WindowRegion};

    const SUBCLASS_ID: usize = 0x514A_5442;

    /// 宿主每帧更新的标题栏命中数据，供 Win32 子类钩子在 `WM_NCHITTEST` 里读取。
    pub(crate) struct TitlebarState {
        pub(crate) scale: f32,
        pub(crate) regions: Vec<WindowHit>,
        /// 当前在非客户区按钮上按下的命中码（`HTMINBUTTON` / `HTMAXBUTTON` / `HTCLOSE`）。
        pub(crate) pressed: Option<u32>,
    }

    struct SubclassData {
        state: Arc<Mutex<TitlebarState>>,
        proxy: EventLoopProxy<HostEvent>,
        /// 是否已为非客户区按钮登记过 `TrackMouseEvent`，移到非按钮区时据此清掉悬停。
        tracking_hover: bool,
    }

    pub(crate) struct WindowsTitlebar {
        state: Arc<Mutex<TitlebarState>>,
    }

    impl WindowsTitlebar {
        pub(crate) fn install(
            window: &Window,
            backdrop: Option<Backdrop>,
            proxy: EventLoopProxy<HostEvent>,
        ) -> Option<Self> {
            let hwnd = hwnd_of(window)?;
            let state = Arc::new(Mutex::new(TitlebarState {
                scale: window.scale_factor() as f32,
                regions: Vec::new(),
                pressed: None,
            }));
            let data = Box::into_raw(Box::new(SubclassData {
                state: Arc::clone(&state),
                proxy,
                tracking_hover: false,
            }));
            let installed = unsafe {
                SetWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID, data as usize).as_bool()
            };
            if !installed {
                unsafe { drop(Box::from_raw(data)) };
                tracing::warn!("SetWindowSubclass failed for custom titlebar");
                return None;
            }
            // 开 Mica 时 winit 已经整窗延伸了框架，这里不要覆盖；没开系统材质时留 1px 顶部边框保住 DWM 阴影与细边
            if backdrop.is_none() {
                let margins = MARGINS {
                    cxLeftWidth: 0,
                    cxRightWidth: 0,
                    cyTopHeight: 1,
                    cyBottomHeight: 0,
                };
                let _ = unsafe { DwmExtendFrameIntoClientArea(hwnd, &margins) };
            }
            // 立刻触发一次 WM_NCCALCSIZE，把原生标题栏收起
            let _ = unsafe {
                SetWindowPos(
                    hwnd,
                    None,
                    0,
                    0,
                    0,
                    0,
                    SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                )
            };
            Some(Self { state })
        }

        pub(crate) fn update(&self, scale: f32, regions: Vec<WindowHit>) {
            if let Ok(mut state) = self.state.lock() {
                state.scale = scale;
                state.regions = regions;
            }
        }

        /// 失焦或客户区抬起鼠标时清掉残留的按钮按下态。
        pub(crate) fn clear_pressed(&self) {
            if let Ok(mut state) = self.state.lock() {
                state.pressed = None;
            }
        }
    }

    fn hwnd_of(window: &Window) -> Option<HWND> {
        let handle = window.window_handle().ok()?;
        match handle.as_raw() {
            RawWindowHandle::Win32(win32) => Some(HWND(win32.hwnd.get() as *mut c_void)),
            _ => None,
        }
    }

    fn screen_point(lparam: LPARAM) -> POINT {
        POINT {
            x: (lparam.0 & 0xFFFF) as i16 as i32,
            y: ((lparam.0 >> 16) & 0xFFFF) as i16 as i32,
        }
    }

    fn client_point(hwnd: HWND, lparam: LPARAM) -> (f32, f32) {
        let mut pt = screen_point(lparam);
        let _ = unsafe { ScreenToClient(hwnd, &mut pt) };
        (pt.x as f32, pt.y as f32)
    }

    fn is_caption_button(hit: u32) -> bool {
        matches!(hit, HTMINBUTTON | HTMAXBUTTON | HTCLOSE)
    }

    unsafe extern "system" fn subclass_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _uid: usize,
        data: usize,
    ) -> LRESULT {
        let Some(ctx) = (unsafe { (data as *mut SubclassData).as_mut() }) else {
            return unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) };
        };

        match msg {
            WM_NCCALCSIZE if wparam.0 != 0 => {
                let params = unsafe { &mut *(lparam.0 as *mut NCCALCSIZE_PARAMS) };
                let original_top = params.rgrc[0].top;
                // 让系统按当前 DPI 算好左、右、下三条隐形缩放边，再只把顶边拉回窗口最顶（最大化时扣掉溢出屏幕外的边框）
                let _ = unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) };
                if unsafe { IsZoomed(hwnd).as_bool() } {
                    let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96);
                    let frame_y = unsafe {
                        GetSystemMetricsForDpi(SM_CYSIZEFRAME, dpi)
                            + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi)
                    };
                    params.rgrc[0].top = original_top + frame_y;
                } else {
                    params.rgrc[0].top = original_top;
                }
                return LRESULT(0);
            }
            WM_NCHITTEST => {
                // 左、右、下三条边框直接交给系统算，只有落在客户区时再细分顶边缩放 / 拖动区 / 三个按钮
                let default = unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) };
                if default.0 != HTCLIENT as isize {
                    return default;
                }
                let (x, y) = client_point(hwnd, lparam);
                let zoomed = unsafe { IsZoomed(hwnd).as_bool() };
                let (scale, region) = match ctx.state.lock() {
                    Ok(state) => (state.scale, hit_region(&state.regions, x, y)),
                    Err(_) => (1.0, None),
                };
                if !zoomed {
                    let top_border = (5.0 * scale).round().max(4.0);
                    if y < top_border {
                        let corner = (10.0 * scale).round().max(8.0);
                        let mut rect = RECT::default();
                        if unsafe { GetClientRect(hwnd, &mut rect) }.is_ok() {
                            let width = rect.right as f32;
                            if x < corner {
                                return LRESULT(HTTOPLEFT as isize);
                            }
                            if x >= width - corner {
                                return LRESULT(HTTOPRIGHT as isize);
                            }
                        }
                        return LRESULT(HTTOP as isize);
                    }
                }
                return match region {
                    Some(WindowRegion::Drag) => LRESULT(HTCAPTION as isize),
                    Some(WindowRegion::Minimize) => LRESULT(HTMINBUTTON as isize),
                    Some(WindowRegion::Maximize) => LRESULT(HTMAXBUTTON as isize),
                    Some(WindowRegion::Close) => LRESULT(HTCLOSE as isize),
                    None => LRESULT(HTCLIENT as isize),
                };
            }
            WM_NCMOUSEMOVE => {
                let hit = wparam.0 as u32;
                if is_caption_button(hit) {
                    if !ctx.tracking_hover {
                        let mut tme = TRACKMOUSEEVENT {
                            cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                            dwFlags: TME_LEAVE | TME_NONCLIENT,
                            hwndTrack: hwnd,
                            dwHoverTime: 0,
                        };
                        let _ = unsafe { TrackMouseEvent(&mut tme) };
                        ctx.tracking_hover = true;
                    }
                    let (x, y) = client_point(hwnd, lparam);
                    let _ = ctx.proxy.send_event(HostEvent::CaptionHover(x, y));
                } else if ctx.tracking_hover {
                    ctx.tracking_hover = false;
                    let _ = ctx.proxy.send_event(HostEvent::CaptionLeave);
                }
            }
            WM_NCMOUSELEAVE => {
                if ctx.tracking_hover {
                    ctx.tracking_hover = false;
                    let _ = ctx.proxy.send_event(HostEvent::CaptionLeave);
                }
            }
            WM_NCLBUTTONDOWN => {
                let hit = wparam.0 as u32;
                if is_caption_button(hit) {
                    // 吞掉默认处理：系统默认实现会在客户区上方画出旧样式按钮
                    if let Ok(mut state) = ctx.state.lock() {
                        state.pressed = Some(hit);
                    }
                    let (x, y) = client_point(hwnd, lparam);
                    let _ = ctx.proxy.send_event(HostEvent::CaptionDown(x, y));
                    return LRESULT(0);
                }
            }
            WM_NCLBUTTONUP => {
                let hit = wparam.0 as u32;
                if is_caption_button(hit) {
                    let matched = ctx
                        .state
                        .lock()
                        .ok()
                        .and_then(|mut state| state.pressed.take())
                        == Some(hit);
                    let _ = ctx.proxy.send_event(HostEvent::CaptionUp);
                    if matched {
                        match hit {
                            HTMINBUTTON => {
                                let _ = unsafe { ShowWindow(hwnd, SW_MINIMIZE) };
                            }
                            HTMAXBUTTON => {
                                let cmd = if unsafe { IsZoomed(hwnd).as_bool() } {
                                    SW_RESTORE
                                } else {
                                    SW_MAXIMIZE
                                };
                                let _ = unsafe { ShowWindow(hwnd, cmd) };
                            }
                            HTCLOSE => {
                                let _ = unsafe {
                                    PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0))
                                };
                            }
                            _ => {}
                        }
                    }
                    return LRESULT(0);
                }
            }
            WM_NCDESTROY => {
                let _ = unsafe { RemoveWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID) };
                let result = unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) };
                unsafe { drop(Box::from_raw(data as *mut SubclassData)) };
                return result;
            }
            _ => {}
        }

        unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
    }
}

#[cfg(target_os = "windows")]
pub(crate) use win32::WindowsTitlebar;

#[cfg(test)]
mod tests {
    use super::hit_region;
    use crate::ui::{WindowHit, WindowRegion};

    fn hit(rect: (f32, f32, f32, f32), region: Option<WindowRegion>) -> WindowHit {
        WindowHit { rect, region }
    }

    #[test]
    fn topmost_interactive_element_overrides_drag_region() {
        let regions = vec![
            hit((0.0, 0.0, 1000.0, 48.0), Some(WindowRegion::Drag)),
            hit((300.0, 8.0, 400.0, 32.0), None), // 搜索框
            hit((862.0, 0.0, 46.0, 48.0), Some(WindowRegion::Minimize)),
            hit((908.0, 0.0, 46.0, 48.0), Some(WindowRegion::Maximize)),
            hit((954.0, 0.0, 46.0, 48.0), Some(WindowRegion::Close)),
        ];
        assert_eq!(hit_region(&regions, 80.0, 20.0), Some(WindowRegion::Drag));
        assert_eq!(hit_region(&regions, 450.0, 20.0), None);
        assert_eq!(
            hit_region(&regions, 880.0, 20.0),
            Some(WindowRegion::Minimize)
        );
        assert_eq!(
            hit_region(&regions, 920.0, 20.0),
            Some(WindowRegion::Maximize)
        );
        assert_eq!(hit_region(&regions, 980.0, 20.0), Some(WindowRegion::Close));
        assert_eq!(hit_region(&regions, 80.0, 80.0), None);
    }

    #[test]
    fn modal_overlay_blocks_titlebar_hit() {
        let regions = vec![
            hit((0.0, 0.0, 1000.0, 48.0), Some(WindowRegion::Drag)),
            hit((954.0, 0.0, 46.0, 48.0), Some(WindowRegion::Close)),
            hit((0.0, 0.0, 1000.0, 680.0), None), // 模态遮罩盖在最上层
        ];
        assert_eq!(hit_region(&regions, 80.0, 20.0), None);
        assert_eq!(hit_region(&regions, 980.0, 20.0), None);
    }
}
