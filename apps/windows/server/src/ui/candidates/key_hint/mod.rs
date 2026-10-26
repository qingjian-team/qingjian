//! 键位提示窗口：候选窗旁一块不抢焦点、点击穿透的分层窗口，键盘由青简渲染器画成位图贴上去。
//! 跟着候选窗显示与收起（[`super::CandidateWindow`] 管）；没开青简渲染器（GDI 画法）时不显示。
//! 键盘内容在 [`keyboard`]，摆在哪在 [`place`]，与 macOS 壳 `candidates::key_hint` 同一套规则。

mod keyboard;
mod place;

use std::cell::{Cell, RefCell};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, SW_HIDE, SW_SHOWNA, ShowWindow, WNDCLASSEXW,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};
use windows::core::{PCWSTR, Result, w};

use qingjian_render::{FontLibrary, Keyboard, Renderer, Theme, system_fonts};

use self::place::Side;
use super::super::layered;
use super::super::monitor;
use super::super::window_class::WindowClass;
use crate::dispatch::{KeyHintContent, KeyHintView};

const CLASS_NAME: PCWSTR = w!("QingjianKeyHintWindow");
static CLASS: WindowClass = WindowClass::new();

pub(super) struct KeyHintWindow {
    hwnd: HWND,

    /// 键盘用自己的渲染器：与候选窗共用会打断候选窗的过渡动画。第一次显示时才建（加载字体库要几十毫秒，
    /// 不开键位提示的人不花这个钱）；建不成为 `Some(None)`，不再试。
    renderer: RefCell<Option<Option<Renderer>>>,

    /// 字根图换成的键盘：字形解析一次，留着复用。
    roots: RefCell<Option<Keyboard>>,

    /// 这一段显示里用的那一侧，收起时忘掉；下次显示先试它，键盘不随光标左右跳。
    side: Cell<Option<Side>>,
}

impl KeyHintWindow {
    pub(super) fn new() -> Result<Self> {
        CLASS.ensure(|| WNDCLASSEXW {
            lpfnWndProc: Some(wndproc),
            hInstance: super::super::module_handle(),
            lpszClassName: CLASS_NAME,
            ..Default::default()
        })?;
        // TRANSPARENT：鼠标穿透到底下的应用，键盘只是看的
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TOOLWINDOW
                    | WS_EX_TOPMOST
                    | WS_EX_NOACTIVATE
                    | WS_EX_TRANSPARENT,
                CLASS_NAME,
                w!("青简键位提示"),
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
        Ok(Self {
            hwnd,
            renderer: RefCell::new(None),
            roots: RefCell::new(None),
            side: Cell::new(None),
        })
    }

    /// 画好键盘贴在候选窗旁边：`candidate` 是候选窗内容区，`caret` 是光标行，都是屏幕物理像素。
    /// 哪儿都放不下就收起，这次不显示。
    pub(super) fn show(
        &self,
        view: &KeyHintView,
        theme: &Theme,
        dark: bool,
        dpi: u32,
        candidate: RECT,
        caret: RECT,
    ) {
        let keyboard = match &view.content {
            KeyHintContent::Shuangpin(hint) => keyboard::shuangpin(hint),
            KeyHintContent::Roots(chart) => self
                .roots
                .borrow_mut()
                .get_or_insert_with(|| keyboard::roots(chart))
                .clone(),
        };
        let scale = dpi.max(96) as f32 / 96.0;
        let rendered = {
            let mut renderer = self.renderer.borrow_mut();
            let Some(renderer) = renderer.get_or_insert_with(|| new_renderer(theme)) else {
                return;
            };
            renderer.render_keyboard(&keyboard, &theme.with_dark(dark), scale, view.zoom)
        };
        let rendered = match rendered {
            Ok(rendered) => rendered,
            Err(error) => {
                tracing::warn!(%error, "键位提示渲染失败");
                self.hide();
                return;
            }
        };
        let size = (
            rendered.content_width as i32,
            rendered.content_height as i32,
        );
        let work = monitor::work_area_near(POINT {
            x: caret.left,
            y: caret.top,
        });
        let gap = (place::GAP * dpi as i32 / 96).max(1);
        let Some(((x, y), side)) = place::place(size, candidate, caret, work, gap, self.side.get())
        else {
            tracing::debug!(?candidate, ?caret, "键位提示放不下，这次不显示");
            self.hide();
            return;
        };
        self.side.set(Some(side));
        let position = (x - rendered.content_x as i32, y - rendered.content_y as i32);
        let alpha = (view.opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
        if layered::present_with_alpha(self.hwnd, &rendered.pixmap, position, alpha).is_ok() {
            let _ = unsafe { ShowWindow(self.hwnd, SW_SHOWNA) };
        } else {
            self.hide();
        }
    }

    pub(super) fn hide(&self) {
        let _ = unsafe { ShowWindow(self.hwnd, SW_HIDE) };
        self.side.set(None);
    }

    /// 主题换了：新主题的字体要装进键盘自己的渲染器。
    pub(super) fn set_theme(&self, theme: &Theme) {
        if let Some(Some(renderer)) = self.renderer.borrow_mut().as_mut() {
            renderer.load_theme_fonts(theme, system_fonts::family_files);
        }
    }
}

impl Drop for KeyHintWindow {
    fn drop(&mut self) {
        let _ = unsafe { DestroyWindow(self.hwnd) };
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
            tracing::warn!(%error, "键位提示的字体库加载失败，不显示键位提示");
            None
        }
    }
}

/// 分层窗口无需 `WM_PAINT`，鼠标也穿透，全交默认处理。
unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}
