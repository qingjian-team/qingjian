//! 云联想轮询定时器：组句期间每隔一小段时间向 Server 拉一次异步结果（云端候选 / 整句补全）。
//!
//! 为什么要它：每次按键都会提交一次新的云联想请求，云端结果几百毫秒后才回，那时用户往往已停下打字，
//! 没有新按键来「顺手收一次」。所以在 TSF 线程上挂一个 `WM_TIMER`，传输仍是一问一答，
//! 不需要 Server 主动推、也不需要 DLL 侧独立读循环。
//!
//! 定时器挂在一个隐藏的消息窗口上，与按键处理同在 STA 消息泵上跑：按键在 `client.key` 里同步等管道时
//! 消息泵不转，定时器插不进来。

use std::cell::RefCell;
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use windows::Win32::Foundation::{E_FAIL, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, HWND_MESSAGE, KillTimer, SetTimer,
    WINDOW_EX_STYLE, WINDOW_STYLE, WM_TIMER, WNDCLASSEXW,
};
use windows::core::{Error, PCWSTR, Result, w};

use super::composition::Shared;
use super::log::log;
use super::service::SharedClient;
use super::window_class::WindowClass;

const CLASS_NAME: PCWSTR = w!("QingjianPollWindow");
static CLASS: WindowClass = WindowClass::new();

const TIMER_ID: usize = 1;

/// 轮询间隔（毫秒）。
const INTERVAL_MS: u32 = 80;

/// 定时器回调要用的东西。
struct PollContext {
    /// 与 `TextService` 共享的引擎客户端。
    engine: SharedClient,

    /// 组句与候选窗口状态。
    shared: Rc<Shared>,
}

thread_local! {
    /// 本线程活着的定时器：消息窗口 → 回调上下文。窗口过程按 HWND 查；查不到（已析构）就忽略这一拍。
    static TIMERS: RefCell<HashMap<isize, Rc<PollContext>>> = RefCell::new(HashMap::new());
}

/// 承载轮询定时器的隐藏消息窗口。`Drop` 里停表、销毁窗口、注销上下文。
pub(crate) struct PollTimer {
    hwnd: HWND,
}

impl PollTimer {
    /// 失败（类注册 / 建窗口 / 挂定时器）返回 `Err`，调用方降级为只在按键时收云结果。
    pub(crate) fn new(engine: SharedClient, shared: Rc<Shared>) -> Result<Self> {
        CLASS.ensure(|| WNDCLASSEXW {
            lpfnWndProc: Some(wndproc),
            hInstance: super::dll_instance(),
            lpszClassName: CLASS_NAME,
            ..Default::default()
        })?;
        // SAFETY: 类已注册；HWND_MESSAGE 建纯消息窗口。
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                CLASS_NAME,
                w!(""),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                Some(super::dll_instance()),
                None,
            )?
        };
        let timer = Self { hwnd };
        // SAFETY: hwnd 有效；TIMERPROC 传 None 让 WM_TIMER 走窗口过程。
        if unsafe { SetTimer(Some(hwnd), TIMER_ID, INTERVAL_MS, None) } == 0 {
            return Err(Error::from(E_FAIL));
        }
        TIMERS.with(|timers| {
            timers
                .borrow_mut()
                .insert(hwnd.0 as isize, Rc::new(PollContext { engine, shared }))
        });
        Ok(timer)
    }
}

impl Drop for PollTimer {
    fn drop(&mut self) {
        TIMERS.with(|timers| timers.borrow_mut().remove(&(self.hwnd.0 as isize)));
        // SAFETY: hwnd 与 timer 由本对象建，只在这里销毁。
        unsafe {
            let _ = KillTimer(Some(self.hwnd), TIMER_ID);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_TIMER {
        // 先 clone 出来放开借用：轮询会重绘候选窗口，别在持有表的借用时做。
        let context = TIMERS.with(|timers| timers.borrow().get(&(hwnd.0 as isize)).cloned());
        if let Some(context) = context {
            // 从消息泵调进来：panic 不能越过 FFI。
            let _ = catch_unwind(AssertUnwindSafe(|| poll_once(&context)));
        }
        return LRESULT(0);
    }
    // SAFETY: 其余消息交默认处理。
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// 只在组句中拉；引擎正被按键处理借用时跳过这一拍；连接坏了断开。
fn poll_once(context: &PollContext) {
    if !context.shared.composing() {
        return;
    }
    let Ok(mut guard) = context.engine.try_borrow_mut() else {
        return;
    };
    let Some(client) = guard.as_mut() else {
        return;
    };
    // 拉一次即可触发 Server 收云端结果并自绘重画候选窗口；DLL 不再据回帧重绘（窗口在 Server 进程）。
    match client.poll() {
        Ok(_frame) => {}
        Err(error) => {
            log(&format!("云联想轮询失败，断开，下一键重连: {error}"));
            *guard = None;
            context.shared.end_composing();
        }
    }
}
