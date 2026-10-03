//! 轮询定时器：组句期间每隔一小段时间向 Server 拉一次异步结果（云端候选 / 整句补全）；
//! 没在组句、本线程在前台时，隔几拍向 Server 取一次全局中英模式（别的应用、悬浮状态条可能切过）。
//! 云端结果几百毫秒后才回，那时往往没有新按键来「顺手收一次」，所以在 TSF 线程上挂一个 `WM_TIMER`；
//! 传输仍是一问一答。定时器挂在隐藏的消息窗口上，与按键同在 STA 消息泵上跑。回调上下文在 [`context`]。

mod context;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use windows::Win32::Foundation::{E_FAIL, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClassNameW, GetForegroundWindow,
    GetWindowThreadProcessId, HWND_MESSAGE, KillTimer, SetTimer, WINDOW_EX_STYLE, WINDOW_STYLE,
    WM_TIMER, WNDCLASSEXW,
};
use windows::core::{Error, PCWSTR, Result, w};

use self::context::PollContext;
use super::composition::{Shared, preedit_string};
use super::edit::request_update;
use super::log::log;
use super::service::SharedClient;
use super::window_class::WindowClass;

const CLASS_NAME: PCWSTR = w!("QingjianPollWindow");
static CLASS: WindowClass = WindowClass::new();

const TIMER_ID: usize = 1;
const INTERVAL_MS: u32 = 80;

/// 没在组句时每几拍问一次状态条的切模式请求（320 ms 一次，点了状态条肉眼看不出延迟）。
/// 按键行为设置（切换键、内置英文模式）也跟着这一拍取回，所以设置改完同样是约 320 ms 生效。
const MODE_SYNC_EVERY: u32 = 4;

thread_local! {
    /// 本线程活着的定时器：消息窗口 → 回调上下文。查不到（已析构）就忽略这一拍。
    static TIMERS: RefCell<HashMap<isize, Rc<PollContext>>> = RefCell::new(HashMap::new());
}

/// 承载轮询定时器的隐藏消息窗口；`Drop` 里停表、销毁窗口、注销上下文。
pub(crate) struct PollTimer {
    hwnd: HWND,
}

impl PollTimer {
    /// 失败返回 `Err`，调用方降级为只在按键时收云结果。
    pub(crate) fn new(engine: SharedClient, shared: Rc<Shared>, client_id: u32) -> Result<Self> {
        CLASS.ensure(|| WNDCLASSEXW {
            lpfnWndProc: Some(wndproc),
            hInstance: super::dll_instance(),
            lpszClassName: CLASS_NAME,
            ..Default::default()
        })?;
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
        if unsafe { SetTimer(Some(hwnd), TIMER_ID, INTERVAL_MS, None) } == 0 {
            return Err(Error::from(E_FAIL));
        }
        TIMERS.with(|timers| {
            timers.borrow_mut().insert(
                hwnd.0 as isize,
                Rc::new(PollContext {
                    engine,
                    shared,
                    client_id,
                    ticks: Cell::new(0),
                }),
            )
        });
        Ok(timer)
    }
}

impl Drop for PollTimer {
    fn drop(&mut self) {
        TIMERS.with(|timers| timers.borrow_mut().remove(&(self.hwnd.0 as isize)));
        unsafe {
            let _ = KillTimer(Some(self.hwnd), TIMER_ID);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_TIMER {
        // 先 clone 出来放开表的借用。
        let context = TIMERS.with(|timers| timers.borrow().get(&(hwnd.0 as isize)).cloned());
        if let Some(context) = context {
            // 从消息泵调进来：panic 不能越过 FFI。
            let _ = catch_unwind(AssertUnwindSafe(|| poll_once(&context)));
        }
        return LRESULT(0);
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// 组句中或翻译评审中拉云结果；否则前台时隔几拍问一次切模式（顺路取回按键行为设置）。引擎正被按键处理借用时跳过这一拍；连接坏了断开。
fn poll_once(context: &PollContext) {
    let tick = context.ticks.get().wrapping_add(1);
    context.ticks.set(tick);
    let translating = context.shared.translating();
    if !context.shared.composing() && !translating {
        if tick.is_multiple_of(MODE_SYNC_EVERY) && in_foreground(context) {
            sync_mode(context);
        }
        return;
    }
    let Ok(mut guard) = context.engine.try_borrow_mut() else {
        return;
    };
    let Some(client) = guard.as_mut() else {
        return;
    };
    match client.poll() {
        Ok(reply) => {
            let commit = reply.commit.filter(|text| !text.is_empty());
            // 翻译评审时回空帧 = 翻译已在 Server 侧结束（云端没给译文）。
            if translating && reply.frame.is_empty() && commit.is_none() {
                drop(guard);
                context.shared.set_translating(false);
                context.shared.hide_candidates();
                return;
            }
            let Some(commit) = commit else {
                return;
            };
            drop(guard);
            let Some(doc) = context.shared.last_context() else {
                log(&format!("候选点击上屏没有上下文，丢弃: {commit:?}"));
                context.shared.set_composing(!reply.frame.is_empty());
                return;
            };
            let preedit = if reply.frame.preedit_mode.inline() {
                preedit_string(&reply.frame)
            } else {
                String::new()
            };
            context.shared.set_composing(!reply.frame.is_empty());
            context.shared.set_translating(false);
            if let Err(error) = request_update(
                &doc,
                context.client_id,
                context.engine.clone(),
                context.shared.clone(),
                Some(commit.clone()),
                preedit,
            ) {
                log(&format!("候选点击上屏的编辑会话没被受理: {error}"));
            } else {
                log(&format!("候选点击上屏: {commit:?}"));
            }
        }
        Err(error) => {
            log(&format!("云联想轮询失败，断开，下一键重连: {error}"));
            *guard = None;
            context.shared.end_composing();
        }
    }
}

/// 本进程是不是在前台：当场看前台窗口属于谁，不只信线程焦点标记（后台进程的标记可能一直不清，
/// 它来取模式会让 Server 以为青简仍是当前输入法）。UWP 应用的前台窗口在 ApplicationFrameHost 进程，那时退回看标记。
fn in_foreground(context: &PollContext) -> bool {
    let window = unsafe { GetForegroundWindow() };
    if window.is_invalid() {
        return false;
    }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(window, Some(&mut pid)) };
    pid == unsafe { GetCurrentProcessId() } || (is_uwp_frame(window) && context.shared.foreground())
}

/// UWP 应用的框架窗口（类名 `ApplicationFrameWindow`）。
fn is_uwp_frame(window: HWND) -> bool {
    let mut name = [0u16; 32];
    let len = unsafe { GetClassNameW(window, &mut name) };
    usize::try_from(len)
        .is_ok_and(|len| String::from_utf16_lossy(&name[..len]) == "ApplicationFrameWindow")
}

/// 取一次全局中英模式，顺路取回 Server 下发的按键行为设置。切模式要落定组句、会借引擎，
/// 所以先放掉借用再切。
fn sync_mode(context: &PollContext) {
    let Ok(mut guard) = context.engine.try_borrow_mut() else {
        return;
    };
    let Some(client) = guard.as_mut() else {
        // 没连着（Server 起得晚 / 重启过）：不等下一键，这一拍就补连（自带退避）。
        drop(guard);
        super::service::on_reconnect_tick();
        return;
    };
    let reply = match client.sync_mode() {
        Ok(reply) => reply,
        Err(error) => {
            log(&format!("同步中英模式失败，断开，下一键重连: {error}"));
            *guard = None;
            return;
        }
    };
    drop(guard);
    // 按键行为设置每一拍都带（DLL 不读配置文件），切换键与内置英文模式开关改完靠它生效。
    super::service::on_input_settings(reply.input);
    super::service::on_indicator_state(reply.indicator);
    if let Some(english) = reply.english {
        super::service::on_mode_sync(english);
    }
}
