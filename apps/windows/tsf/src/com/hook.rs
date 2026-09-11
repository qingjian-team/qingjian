//! 单击 Shift 切中英。TSF 的击键 sink 收不到独立的修饰键（真机 tsf.log 验证：按 Shift 没有任何回调），
//! 所以用一个**线程级** `WH_KEYBOARD` 钩子直接看 Shift 的按下 / 抬起。钩子只装在本 TSF 所在的 UI 线程上，
//! 只看发往本线程窗口的键——本输入法有焦点时才收得到；`Deactivate` 时卸掉。

use core::cell::Cell;

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, HC_ACTION, HHOOK, SetWindowsHookExW, UnhookWindowsHookEx, WH_KEYBOARD,
};

use super::keys::is_shift;
use super::log::log;

thread_local! {
    /// 本线程装的钩子句柄（`HHOOK` 的指针存成 isize）；0 表示没装。
    static HOOK: Cell<isize> = const { Cell::new(0) };

    /// 按下 Shift 后还没有别的键插进来：单独抬起就是一次中英切换。任一非 Shift 键按下会清掉它。
    static SHIFT_ALONE: Cell<bool> = const { Cell::new(false) };
}

/// 在当前线程装 Shift 钩子（已装则先卸）。装不上只记日志：切换失效但不影响打字。
pub(super) fn install() {
    remove();
    // SAFETY: 线程级钩子，回调在本 DLL 内，hmod 传 None，线程用当前线程。
    match unsafe { SetWindowsHookExW(WH_KEYBOARD, Some(hook_proc), None, GetCurrentThreadId()) } {
        Ok(handle) => HOOK.with(|h| h.set(handle.0 as isize)),
        Err(error) => log(&format!("装 Shift 钩子失败: {error}")),
    }
}

/// 卸掉钩子（`Deactivate` 调）。
pub(super) fn remove() {
    let handle = HOOK.with(|h| h.replace(0));
    if handle != 0 {
        // SAFETY: handle 是本线程刚才装的钩子。
        let _ = unsafe { UnhookWindowsHookEx(HHOOK(handle as *mut core::ffi::c_void)) };
    }
    SHIFT_ALONE.with(|s| s.set(false));
}

/// `WH_KEYBOARD` 回调：`lparam` 第 31 位 1=抬起 / 0=按下，第 30 位是按下前的状态（区分自动重复）。
unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let vk = wparam.0 as u32;
        let released = (lparam.0 >> 31) & 1 == 1;
        let was_down = (lparam.0 >> 30) & 1 == 1;
        if released {
            if is_shift(vk) && SHIFT_ALONE.with(|s| s.replace(false)) {
                super::service::on_shift_tap();
            }
        } else if is_shift(vk) {
            // 首次按下（非自动重复）才算候选，别让长按 Shift 反复触发。
            if !was_down {
                SHIFT_ALONE.with(|s| s.set(true));
            }
        } else {
            SHIFT_ALONE.with(|s| s.set(false));
        }
    }
    // SAFETY: 把事件交给链上下一个钩子。
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}
