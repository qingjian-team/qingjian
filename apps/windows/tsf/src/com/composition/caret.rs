//! 候选窗口的定位锚点：组句范围在屏幕上的矩形，拿不到时退到鼠标位置。

use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::UI::TextServices::{ITfComposition, ITfContext};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
use windows::core::BOOL;

/// 组句范围的屏幕矩形（`GetActiveView` + `GetTextExt`）。有些应用给不出（全零 / 空矩形），返回 `None`。
pub(super) fn caret_rect(
    context: &ITfContext,
    ec: u32,
    composition: &ITfComposition,
) -> Option<RECT> {
    let mut rect = RECT::default();
    let mut clipped = BOOL(0);
    // SAFETY: ec 是本会话的读写锁；composition 是活动组句。
    unsafe {
        let range = composition.GetRange().ok()?;
        let view = context.GetActiveView().ok()?;
        view.GetTextExt(ec, &range, &mut rect, &mut clipped).ok()?;
    }
    if rect.right <= rect.left && rect.bottom <= rect.top {
        return None;
    }
    Some(rect)
}

/// 兜底锚点：鼠标处一个零宽、约一行高的矩形。
pub(super) fn mouse_anchor() -> RECT {
    let mut point = POINT::default();
    // SAFETY: point 可写。
    let _ = unsafe { GetCursorPos(&mut point) };
    RECT {
        left: point.x,
        top: point.y,
        right: point.x,
        bottom: point.y + 16,
    }
}
