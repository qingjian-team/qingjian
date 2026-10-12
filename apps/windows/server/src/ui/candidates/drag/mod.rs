//! 只为游戏候选启用顶部拖动；按 HWND 持有状态，窗口析构后移除。

mod positions;
mod state;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use windows::Win32::Foundation::{HWND, LPARAM, POINT};
use windows::Win32::Graphics::Gdi::ClientToScreen;

pub(super) use state::DragState;

thread_local! {
    static STATES: RefCell<HashMap<isize, Rc<DragState>>> = RefCell::new(HashMap::new());
}

pub(super) fn attach(hwnd: HWND) -> Rc<DragState> {
    let state = Rc::new(DragState::new());
    STATES.with(|states| states.borrow_mut().insert(hwnd.0 as isize, state.clone()));
    state
}

pub(super) fn detach(hwnd: HWND) {
    STATES.with(|states| states.borrow_mut().remove(&(hwnd.0 as isize)));
}

pub(super) fn state(hwnd: HWND) -> Option<Rc<DragState>> {
    STATES.with(|states| states.borrow().get(&(hwnd.0 as isize)).cloned())
}

pub(super) fn mouse_points(hwnd: HWND, lparam: LPARAM) -> Option<(POINT, POINT)> {
    let client = POINT {
        x: (lparam.0 & 0xffff) as i16 as i32,
        y: ((lparam.0 >> 16) & 0xffff) as i16 as i32,
    };
    let mut screen = client;
    unsafe { ClientToScreen(hwnd, &mut screen) }
        .as_bool()
        .then_some((client, screen))
}
