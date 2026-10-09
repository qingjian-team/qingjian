//! 候选窗口的点击：只有候选与译词那几块收鼠标（`WM_NCHITTEST` 回 `HTCLIENT`），其余地方回 `HTTRANSPARENT`
//! 穿到下面的应用，主题伸出窗口的装饰、投影都不挡；按下不激活窗口，点中的目标经回调交给 Router。
//! GDI 画法没有点击区域，整窗穿透。

use std::cell::RefCell;

use windows::Win32::Foundation::{LPARAM, LRESULT, POINT};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, HTCLIENT, HTTRANSPARENT, MA_NOACTIVATE,
};

use qingjian_render::{HitRegion, HitTarget};

/// 点中候选或译词时调的回调（UI 线程上调，投给工人线程）。
pub type CandidateClicks = Box<dyn Fn(HitTarget) + Send>;

/// 本线程候选窗口的点击状态（只有一个候选窗口）。
struct Clicks {
    on_click: CandidateClicks,

    /// 最近一次贴的位图里的点击区域（内容区像素）。
    hits: Vec<HitRegion>,

    /// 内容区左上角的屏幕坐标（物理像素）。
    origin: (i32, i32),
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
    CLICKS.with(|clicks| {
        if let Some(clicks) = clicks.borrow().as_ref() {
            (clicks.on_click)(target);
        }
    });
}
