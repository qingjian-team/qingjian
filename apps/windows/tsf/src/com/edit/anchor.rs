//! 候选窗口的定位锚点：组句范围 / 选区在屏幕上的矩形。
//! 游戏（斗阵特攻等）聊天是自绘的，`GetTextExt` 经常给整窗、系统默认右下角或失败，不能一失败就跟鼠标。

use std::mem::{ManuallyDrop, size_of};

use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::Input::Ime::{
    CANDIDATEFORM, CFS_FORCE_POSITION, CFS_POINT, COMPOSITIONFORM, ImmGetCandidateWindow,
    ImmGetCompositionWindow, ImmGetContext, ImmReleaseContext,
};
use windows::Win32::UI::TextServices::{ITfContext, ITfRange, TF_DEFAULT_SELECTION, TF_SELECTION};
use windows::Win32::UI::WindowsAndMessaging::{
    GUITHREADINFO, GetCaretPos, GetClientRect, GetCursorPos, GetForegroundWindow, GetGUIThreadInfo,
    GetWindowRect,
};
use windows::core::BOOL;

use qingjian_platform::protocol::ScreenRect;

/// `range` 的屏幕矩形；应用给不出可用光标时再走回退链。
pub(crate) fn anchor_rect(context: &ITfContext, ec: u32, range: &ITfRange) -> ScreenRect {
    to_screen(resolve(context, ec, Some(range)))
}

/// 插入点（没有选区时是光标，有选区时是选区）的屏幕矩形。「只在候选窗口」模式应用里不放 marked text，
/// 没有组句范围可量，就用它给候选窗口定位。
pub(crate) fn caret_rect(context: &ITfContext, ec: u32) -> ScreenRect {
    to_screen(resolve(context, ec, None))
}

/// 当前选区的范围；`GetSelection` 移交所有权，由调用方释放。
pub(crate) fn selection_range(context: &ITfContext, ec: u32) -> Option<ITfRange> {
    let mut selection = [TF_SELECTION::default()];
    let mut fetched = 0u32;
    unsafe {
        context
            .GetSelection(ec, TF_DEFAULT_SELECTION, &mut selection, &mut fetched)
            .ok()?;
    }
    if fetched == 0 {
        return None;
    }
    unsafe { ManuallyDrop::take(&mut selection[0].range) }
}

/// 没有可量的范围时的锚点：鼠标处一个零宽、约一行高的矩形。
pub(crate) fn mouse_screen_rect() -> ScreenRect {
    to_screen(mouse_anchor())
}

fn resolve(context: &ITfContext, ec: u32, range: Option<&ITfRange>) -> RECT {
    let view = unsafe { context.GetActiveView().ok() };
    let hwnd = view
        .as_ref()
        .and_then(|view| unsafe { view.GetWnd().ok() })
        .filter(|hwnd| !hwnd.is_invalid())
        .or_else(foreground_hwnd);

    if let Some(range) = range
        && let Some(rect) = text_ext(context, ec, range)
    {
        let mapped = hwnd
            .map(|hwnd| map_client_if_needed(hwnd, rect))
            .unwrap_or(rect);
        if plausible_caret(mapped, hwnd.and_then(window_rect)) {
            return mapped;
        }
    }

    if let Some(rect) = gui_caret().filter(|rect| plausible_caret(*rect, hwnd.and_then(window_rect)))
    {
        return rect;
    }
    if let Some(hwnd) = hwnd
        && let Some(rect) = imm_caret(hwnd).filter(|rect| plausible_caret(*rect, window_rect(hwnd)))
    {
        return rect;
    }
    if let Some(hwnd) = hwnd
        && let Some(rect) = gdi_caret(hwnd).filter(|rect| plausible_caret(*rect, window_rect(hwnd)))
    {
        return rect;
    }
    if let Some(hwnd) = hwnd
        && let Some(rect) = mouse_if_inside(hwnd)
    {
        return rect;
    }
    hwnd.and_then(window_input_fallback).unwrap_or_else(mouse_anchor)
}

fn text_ext(context: &ITfContext, ec: u32, range: &ITfRange) -> Option<RECT> {
    let mut rect = RECT::default();
    let mut clipped = BOOL(0);
    unsafe {
        let view = context.GetActiveView().ok()?;
        view.GetTextExt(ec, range, &mut rect, &mut clipped).ok()?;
    }
    nonempty(rect).then_some(rect)
}

fn gui_caret() -> Option<RECT> {
    let mut info = GUITHREADINFO {
        cbSize: size_of::<GUITHREADINFO>() as u32,
        ..Default::default()
    };
    unsafe { GetGUIThreadInfo(0, &mut info).ok()? };
    let hwnd = if !info.hwndCaret.is_invalid() {
        info.hwndCaret
    } else if !info.hwndFocus.is_invalid() {
        info.hwndFocus
    } else {
        return None;
    };
    if !nonempty(info.rcCaret) {
        return None;
    }
    Some(client_rect_to_screen(hwnd, info.rcCaret))
}

fn imm_caret(hwnd: HWND) -> Option<RECT> {
    unsafe {
        let himc = ImmGetContext(hwnd);
        if himc.is_invalid() {
            return None;
        }
        let mut composition = COMPOSITIONFORM::default();
        let has_composition = ImmGetCompositionWindow(himc, &mut composition).as_bool();
        let mut candidate = CANDIDATEFORM::default();
        let has_candidate = ImmGetCandidateWindow(himc, 0, &mut candidate).as_bool();
        let _ = ImmReleaseContext(hwnd, himc);
        if has_composition
            && let Some(rect) = composition_point(hwnd, &composition)
        {
            return Some(rect);
        }
        if has_candidate
            && let Some(rect) = candidate_point(hwnd, &candidate)
        {
            return Some(rect);
        }
    }
    None
}

fn composition_point(hwnd: HWND, form: &COMPOSITIONFORM) -> Option<RECT> {
    point_style(form.dwStyle).then(|| point_to_caret(hwnd, form.ptCurrentPos))
}

fn candidate_point(hwnd: HWND, form: &CANDIDATEFORM) -> Option<RECT> {
    point_style(form.dwStyle).then(|| point_to_caret(hwnd, form.ptCurrentPos))
}

fn point_style(style: u32) -> bool {
    style & (CFS_POINT | CFS_FORCE_POSITION) != 0
}

fn point_to_caret(hwnd: HWND, mut point: POINT) -> RECT {
    let _ = unsafe { ClientToScreen(hwnd, &mut point) };
    RECT {
        left: point.x,
        top: point.y,
        right: point.x,
        bottom: point.y + 16,
    }
}

fn gdi_caret(hwnd: HWND) -> Option<RECT> {
    let mut point = POINT::default();
    unsafe { GetCaretPos(&mut point).ok()? };
    if point.x == 0 && point.y == 0 {
        return None;
    }
    Some(point_to_caret(hwnd, point))
}

fn mouse_if_inside(hwnd: HWND) -> Option<RECT> {
    let window = window_rect(hwnd)?;
    let mut point = POINT::default();
    let _ = unsafe { GetCursorPos(&mut point) };
    if !contains(window, point) {
        return None;
    }
    let caret = mouse_anchor();
    plausible_caret(caret, Some(window)).then_some(caret)
}

/// 游戏常按 Enter 开聊天、鼠标还停在屏幕角落：用窗口左侧中部当最后退路，好过系统默认的右下角。
fn window_input_fallback(hwnd: HWND) -> Option<RECT> {
    let mut client = RECT::default();
    unsafe { GetClientRect(hwnd, &mut client).ok()? };
    if !nonempty(client) {
        return None;
    }
    let point = POINT {
        x: client.left + 24,
        y: client.top + (client.bottom - client.top) / 2,
    };
    Some(point_to_caret(hwnd, point))
}

fn foreground_hwnd() -> Option<HWND> {
    let hwnd = unsafe { GetForegroundWindow() };
    (!hwnd.is_invalid()).then_some(hwnd)
}

fn window_rect(hwnd: HWND) -> Option<RECT> {
    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect).ok()? };
    nonempty(rect).then_some(rect)
}

/// `GetTextExt` 有的应用给客户区坐标。窗口不在屏幕原点、矩形又完全落在客户区内时，当成客户区再换算。
fn map_client_if_needed(hwnd: HWND, rect: RECT) -> RECT {
    let mut client = RECT::default();
    let mut window = RECT::default();
    if unsafe { GetClientRect(hwnd, &mut client) }.is_err()
        || unsafe { GetWindowRect(hwnd, &mut window) }.is_err()
    {
        return rect;
    }
    let in_client = rect.left >= 0
        && rect.top >= 0
        && rect.right <= client.right
        && rect.bottom <= client.bottom;
    let in_window = rect.left >= window.left
        && rect.top >= window.top
        && rect.right <= window.right
        && rect.bottom <= window.bottom;
    if in_client && !in_window {
        let mut top_left = POINT {
            x: rect.left,
            y: rect.top,
        };
        let mut bottom_right = POINT {
            x: rect.right,
            y: rect.bottom,
        };
        let _ = unsafe { ClientToScreen(hwnd, &mut top_left) };
        let _ = unsafe { ClientToScreen(hwnd, &mut bottom_right) };
        RECT {
            left: top_left.x,
            top: top_left.y,
            right: bottom_right.x,
            bottom: bottom_right.y,
        }
    } else {
        rect
    }
}

fn client_rect_to_screen(hwnd: HWND, rect: RECT) -> RECT {
    let mut top_left = POINT {
        x: rect.left,
        y: rect.top,
    };
    let mut bottom_right = POINT {
        x: rect.right,
        y: rect.bottom,
    };
    let _ = unsafe { ClientToScreen(hwnd, &mut top_left) };
    let _ = unsafe { ClientToScreen(hwnd, &mut bottom_right) };
    RECT {
        left: top_left.x,
        top: top_left.y,
        right: bottom_right.x,
        bottom: bottom_right.y,
    }
}

fn mouse_anchor() -> RECT {
    let mut point = POINT::default();
    let _ = unsafe { GetCursorPos(&mut point) };
    RECT {
        left: point.x,
        top: point.y,
        right: point.x,
        bottom: point.y + 16,
    }
}

fn to_screen(rect: RECT) -> ScreenRect {
    ScreenRect {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    }
}

fn nonempty(rect: RECT) -> bool {
    rect.right > rect.left || rect.bottom > rect.top
}

fn contains(rect: RECT, point: POINT) -> bool {
    point.x >= rect.left && point.x < rect.right && point.y >= rect.top && point.y < rect.bottom
}

/// 整窗矩形、系统默认的右下角 IME 位置都不能当光标。
fn plausible_caret(caret: RECT, window: Option<RECT>) -> bool {
    if !nonempty(caret) {
        return false;
    }
    let Some(window) = window else {
        return true;
    };
    let win_w = i64::from(window.right.saturating_sub(window.left));
    let win_h = i64::from(window.bottom.saturating_sub(window.top));
    if win_w <= 0 || win_h <= 0 {
        return true;
    }
    let caret_w = i64::from((caret.right - caret.left).max(1));
    let caret_h = i64::from((caret.bottom - caret.top).max(1));
    if caret_w * 2 > win_w && caret_h * 2 > win_h {
        return false;
    }
    const CORNER: i32 = 96;
    let near_right = caret.left >= window.right - CORNER - (caret.right - caret.left).max(0);
    let near_bottom = caret.top >= window.bottom - CORNER - (caret.bottom - caret.top).max(16);
    !(near_right && near_bottom)
}

#[cfg(test)]
mod tests {
    use super::plausible_caret;
    use windows::Win32::Foundation::RECT;

    fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
        RECT {
            left,
            top,
            right,
            bottom,
        }
    }

    #[test]
    fn rejects_a_full_window_dummy() {
        let window = rect(0, 0, 1920, 1080);
        assert!(!plausible_caret(window, Some(window)));
    }

    #[test]
    fn rejects_the_default_ime_corner() {
        let window = rect(0, 0, 1920, 1080);
        let corner = rect(1840, 1040, 1840, 1056);
        assert!(!plausible_caret(corner, Some(window)));
    }

    #[test]
    fn accepts_a_chat_box_on_the_left() {
        let window = rect(0, 0, 1920, 1080);
        let chat = rect(80, 430, 82, 452);
        assert!(plausible_caret(chat, Some(window)));
    }
}
