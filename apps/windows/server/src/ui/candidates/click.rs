//! 候选窗口的点击：只有候选与译词那几块收鼠标（`WM_NCHITTEST` 回 `HTCLIENT`），其余地方回 `HTTRANSPARENT`
//! 穿到下面的应用，主题伸出窗口的装饰、投影都不挡；按下不激活窗口，点中的目标经回调交给 Router。
//! GDI 画法没有点击区域，整窗穿透。右键候选要菜单，菜单里点中的项也经这个回调交回。
//! 主题里的翻页箭头点了翻页；滚轮在候选上滚动也翻页（系统缺省把滚轮发给光标下的窗口，候选窗不用抢焦点）。

use std::cell::RefCell;

use windows::Win32::Foundation::{LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, HTCLIENT, HTTRANSPARENT, MA_NOACTIVATE, WHEEL_DELTA,
};

use qingjian_render::{HitRegion, HitTarget};

pub(crate) use crate::dispatch::CandidateMouse;

/// 候选窗口上的鼠标操作回调（UI 线程上调，投给工人线程）。
pub type CandidateClicks = Box<dyn Fn(CandidateMouse) + Send>;

/// 本线程候选窗口的点击状态（只有一个候选窗口）。
struct Clicks {
    on_click: CandidateClicks,

    /// 最近一次贴的位图里的点击区域（内容区像素）。
    hits: Vec<HitRegion>,

    /// 内容区左上角的屏幕坐标（物理像素）。
    origin: (i32, i32),

    /// 滚轮还没凑够一格的余量：触控板一次只给几十，累加到 120 才翻一页。
    wheel: i32,
}

thread_local! {
    static CLICKS: RefCell<Option<Clicks>> = const { RefCell::new(None) };
}

/// 建窗口时登记回调。
pub(super) fn install(on_click: CandidateClicks) {
    CLICKS.with(|clicks| {
        *clicks.borrow_mut() = Some(Clicks {
            on_click,
            hits: Vec::new(),
            origin: (0, 0),
            wheel: 0,
        });
    });
}

/// 每次贴位图后换上新的点击区域；GDI 画法与收起时传空。
pub(super) fn set_hits(hits: Vec<HitRegion>, origin: (i32, i32)) {
    CLICKS.with(|clicks| {
        if let Some(clicks) = clicks.borrow_mut().as_mut() {
            clicks.hits = hits;
            clicks.origin = origin;
        }
    });
}

/// 屏幕上这一点落在哪个目标上。
fn target_at(x: i32, y: i32) -> Option<HitTarget> {
    CLICKS.with(|clicks| {
        let clicks = clicks.borrow();
        let clicks = clicks.as_ref()?;
        let (x, y) = ((x - clicks.origin.0) as f32, (y - clicks.origin.1) as f32);
        clicks
            .hits
            .iter()
            .find(|region| region.contains(x, y))
            .map(|region| region.target)
    })
}

/// `WM_NCHITTEST`：`lparam` 是屏幕坐标（有符号 16 位）。
pub(super) fn hit_test(lparam: LPARAM) -> LRESULT {
    let x = (lparam.0 & 0xFFFF) as i16 as i32;
    let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
    if target_at(x, y).is_some() {
        LRESULT(HTCLIENT as isize)
    } else {
        LRESULT(HTTRANSPARENT as isize)
    }
}

pub(super) fn mouse_activate() -> LRESULT {
    LRESULT(MA_NOACTIVATE as isize)
}

/// `WM_LBUTTONDOWN`：按当前光标的屏幕位置认目标，交给回调。
pub(super) fn button_down() {
    let mut point = POINT::default();
    if unsafe { GetCursorPos(&mut point) }.is_err() {
        return;
    }
    let Some(target) = target_at(point.x, point.y) else {
        return;
    };
    tracing::debug!(?target, "点击候选窗口");
    send(CandidateMouse::Click(target));
}

/// `WM_RBUTTONUP`：右键在候选或译词上就要菜单。
pub(super) fn right_button_up() {
    let mut point = POINT::default();
    if unsafe { GetCursorPos(&mut point) }.is_err() {
        return;
    }
    let offset = match target_at(point.x, point.y) {
        Some(
            HitTarget::Candidate(offset)
            | HitTarget::Translation {
                candidate: offset, ..
            },
        ) => offset,
        Some(HitTarget::MenuItem(_) | HitTarget::Page(_)) | None => return,
    };
    send(CandidateMouse::Menu {
        offset,
        point: (point.x, point.y),
    });
}

/// `WM_MOUSEWHEEL`：凑够一格翻一页，往下滚是下一页。
pub(super) fn wheel(wparam: WPARAM) {
    let delta = ((wparam.0 >> 16) & 0xFFFF) as i16 as i32;
    let pages = CLICKS.with(|clicks| {
        let mut clicks = clicks.borrow_mut();
        let clicks = clicks.as_mut()?;
        let total = clicks.wheel + delta;
        let step = WHEEL_DELTA as i32;
        let pages = total / step;
        clicks.wheel = total - pages * step;
        Some(pages)
    });
    if let Some(pages) = pages.filter(|&pages| pages != 0) {
        send(CandidateMouse::Page(-pages as isize));
    }
}

/// 交给登记的回调。
pub(super) fn send(event: CandidateMouse) {
    CLICKS.with(|clicks| {
        if let Some(clicks) = clicks.borrow().as_ref() {
            (clicks.on_click)(event);
        }
    });
}
