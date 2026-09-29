//! 候选窗口的定位锚点：组句起点 / 选区在屏幕上的矩形，拿不到时退到鼠标位置。
//!
//! 微信 4.x 对「非空范围」的 GetTextExt 只回终点处 1 像素残片（首键却给整行高，实测
//! (…,1062,…,1082) 对 (…,1063,…,1064)），锚点因此逐键跳、贴下方还盖住输入行——组句
//! 一律折叠到起点再量（对齐 weasel），残片按上次的正常行高补回整行。

use std::mem::ManuallyDrop;
use std::sync::atomic::{AtomicI32, Ordering};

use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::UI::TextServices::{
    ITfContext, ITfRange, TF_ANCHOR_START, TF_DEFAULT_SELECTION, TF_SELECTION,
};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
use windows::core::BOOL;

use qingjian_platform::protocol::ScreenRect;

/// `range` 的屏幕矩形，拿不到（有些应用给全零 / 空矩形）退到鼠标处；残片矩形补回整行。
/// 每次取锚都留一行日志：候选框「上下乱跳」的排查全靠它（微信首键矩形坏掉一类的应用怪癖要有数据才能定）。
pub(crate) fn anchor_rect(context: &ITfContext, ec: u32, range: &ITfRange) -> ScreenRect {
    let (mut rect, source) = collapsed_rect(context, ec, range)
        .map(|rect| (rect, "起点"))
        .or_else(|| range_rect(context, ec, range).map(|rect| (rect, "整段")))
        .unwrap_or_else(|| (mouse_anchor(), "鼠标回退"));
    let repaired = repair_degenerate(&mut rect);
    crate::com::log::log(&format!(
        "候选锚点 来源={source} rect=({},{},{},{}){}",
        rect.left,
        rect.top,
        rect.right,
        rect.bottom,
        if repaired {
            "（残片已补整行）"
        } else {
            ""
        },
    ));
    to_screen(rect)
}

/// 组句**起点**的矩形：克隆范围折叠到起点再量。组句期间起点不动，锚点逐键稳定；
/// 微信对空范围（光标点）的 GetTextExt 正常，正好绕开非空范围的残片实现（小狼毫
/// Composition.cpp 同款策略）。克隆再折叠，不动组句本体的范围。
fn collapsed_rect(context: &ITfContext, ec: u32, range: &ITfRange) -> Option<RECT> {
    let clone = unsafe { range.Clone() }.ok()?;
    unsafe { clone.Collapse(ec, TF_ANCHOR_START) }.ok()?;
    range_rect(context, ec, &clone)
}

/// 插入点（没有选区时是光标，有选区时是选区）的屏幕矩形。「只在候选窗口」模式应用里不放 marked text，
/// 没有组句范围可量，就用它给候选窗口定位。
pub(crate) fn caret_rect(context: &ITfContext, ec: u32) -> ScreenRect {
    match selection_range(context, ec) {
        Some(range) => anchor_rect(context, ec, &range),
        None => mouse_screen_rect(),
    }
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

/// 上一条正常锚点矩形量到的行高（物理像素），见 `repair_degenerate`。
static LAST_LINE_HEIGHT: AtomicI32 = AtomicI32::new(0);

/// 行高的可信区间：低于下限的是光标残片，高于上限的多半是整个编辑框，都不能当「一行」记。
const MIN_LINE_HEIGHT: i32 = 8;
const MAX_LINE_HEIGHT: i32 = 200;

/// 残片矩形按「顶边＝行顶、行高＝上次的正常行高」补回整行：同一行内摆放不再翻转，
/// 残片底边也不会把窗口顶进输入行。返回是否补过。
fn repair_degenerate(rect: &mut RECT) -> bool {
    let height = rect.bottom - rect.top;
    if (MIN_LINE_HEIGHT..=MAX_LINE_HEIGHT).contains(&height) {
        LAST_LINE_HEIGHT.store(height, Ordering::Relaxed);
        return false;
    }
    let remembered = LAST_LINE_HEIGHT.load(Ordering::Relaxed);
    if height < MIN_LINE_HEIGHT && remembered >= MIN_LINE_HEIGHT {
        rect.bottom = rect.top + remembered;
        return true;
    }
    false
}

fn range_rect(context: &ITfContext, ec: u32, range: &ITfRange) -> Option<RECT> {
    let mut rect = RECT::default();
    let mut clipped = BOOL(0);
    unsafe {
        let view = context.GetActiveView().ok()?;
        view.GetTextExt(ec, range, &mut rect, &mut clipped).ok()?;
    }
    (rect.right > rect.left || rect.bottom > rect.top).then_some(rect)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(top: i32, bottom: i32) -> RECT {
        RECT {
            left: 100,
            top,
            right: 103,
            bottom,
        }
    }

    // 微信输入框实测形状：首键整行高 20，之后每键 1 像素残片；补回后同一行底边一致，
    // 摆放判定不再翻转。
    #[test]
    fn fragment_is_repaired_to_last_line_height() {
        LAST_LINE_HEIGHT.store(0, Ordering::Relaxed);
        let mut orphan = rect(1063, 1064);
        assert!(!repair_degenerate(&mut orphan)); // 还没记过行高，照用残片
        assert_eq!(orphan.bottom, 1064);

        let mut first = rect(1062, 1082);
        assert!(!repair_degenerate(&mut first)); // 整行高，记下不补
        assert_eq!(first.bottom, 1082);

        let mut later = rect(1063, 1064);
        assert!(repair_degenerate(&mut later)); // 同行的残片补回整行
        assert_eq!(later.bottom, 1083);
    }

    // 整框矩形（整个编辑框那么高）不能当行高记，也不许被补。
    #[test]
    fn oversized_rect_is_neither_stored_nor_repaired() {
        LAST_LINE_HEIGHT.store(0, Ordering::Relaxed);
        let mut whole = rect(0, 900);
        assert!(!repair_degenerate(&mut whole));
        assert_eq!(whole.bottom, 900);
        assert_eq!(LAST_LINE_HEIGHT.load(Ordering::Relaxed), 0);
    }
}
