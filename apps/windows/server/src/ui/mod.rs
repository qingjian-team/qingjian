//! 候选窗口的自绘线程（Server 进程内）。
//!
//! 候选窗口从前是 TSF DLL 在应用进程里自绘，普通置顶窗被微软商店 / 任务栏搜索这些**更高 z-band** 的宿主盖住。
//! 把渲染搬到 Server 进程（[`candidates`]，与旧 DLL 同一套 GDI 分层窗口代码），再配合 uiAccess 签名（第 2 段），
//! `SetWindowPos(HWND_TOPMOST)` 才能升进 UIAccess 高带盖过它们。
//!
//! 窗口是线程亲和的：HWND 只在这条 UI 线程上碰。[`Router`](crate::dispatch::Router) 在工人线程上产出
//! [`Frame`] 与光标矩形，经通道 + `PostThreadMessageW` 唤醒把命令 marshal 过来，UI 线程排空队列后应用。

mod candidates;
mod window_class;

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use windows::Win32::Foundation::{E_FAIL, HINSTANCE, LPARAM, RECT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, MSG, PostThreadMessageW, TranslateMessage, WM_APP,
};
use windows::core::{Error, Result};

use qingjian_platform::protocol::{Frame, ScreenRect};

use self::candidates::CandidateWindow;
use crate::dispatch::CandidateSink;

/// 唤醒 UI 线程去排空命令队列的线程消息。
const WM_WAKE: u32 = WM_APP;

/// 交给 UI 线程执行的候选窗口命令。`Frame` 较大，装箱避免枚举体过胖。
enum UiCommand {
    /// 摆到 `rect`（组句范围的屏幕矩形）下方并按 `frame` 重绘。
    Show(Box<(Frame, ScreenRect)>),

    /// 收起候选窗口。
    Hide,
}

/// 候选窗口自绘线程的句柄。发命令 = 投进通道 + 投递一条 `WM_WAKE` 唤醒消息泵。
/// UI 线程 detached，随 Server 进程存活到退出（无需 join）。
pub struct CandidateUi {
    /// 命令通道的发送端。
    sender: Sender<UiCommand>,

    /// UI 线程 id，`PostThreadMessageW` 据它唤醒。
    thread_id: u32,
}

impl CandidateUi {
    /// 起候选窗口 UI 线程：注册窗口类、建隐藏的分层窗口、跑消息循环。
    /// 等 UI 线程建好窗口再返回，失败（类注册 / 建窗口 / 起线程）返回 `Err`，调用方退化为不画候选窗口。
    pub fn spawn() -> Result<Self> {
        // 报告用 `Option<u32>`（成功带线程 id）而非 Result，免得 windows Error 跨线程；建窗口失败在 UI 线程记日志。
        let (ready_tx, ready_rx) = mpsc::channel::<Option<u32>>();
        let (command_tx, command_rx) = mpsc::channel::<UiCommand>();
        // UI 线程 detached（不留 JoinHandle）：随进程存活，退出时随之结束。
        thread::Builder::new()
            .name("qingjian-candidates".to_owned())
            .spawn(move || run(command_rx, &ready_tx))
            .map_err(|_| Error::from(E_FAIL))?;
        match ready_rx.recv() {
            Ok(Some(thread_id)) => Ok(Self {
                sender: command_tx,
                thread_id,
            }),
            // 建窗口失败（已在 UI 线程记日志）或线程没跑到报告就没了。
            _ => Err(Error::from(E_FAIL)),
        }
    }

    /// 投递一条命令并唤醒 UI 线程。线程已退出（通道断）时静默丢弃。
    fn post(&self, command: UiCommand) {
        if self.sender.send(command).is_ok() {
            // SAFETY: 只投递不带指针的 WM_WAKE 线程消息，唤醒 UI 线程的 GetMessage。
            let _ = unsafe { PostThreadMessageW(self.thread_id, WM_WAKE, WPARAM(0), LPARAM(0)) };
        }
    }
}

impl CandidateSink for CandidateUi {
    fn show(&self, frame: Frame, rect: ScreenRect) {
        self.post(UiCommand::Show(Box::new((frame, rect))));
    }

    fn hide(&self) {
        self.post(UiCommand::Hide);
    }
}

/// 本进程 exe 的模块句柄（注册窗口类 / 建窗口用）。取不到回落 null，系统按当前进程处理。
pub(super) fn module_handle() -> HINSTANCE {
    // SAFETY: None 取当前进程 exe 的模块句柄。
    let module = unsafe { GetModuleHandleW(None) }.unwrap_or_default();
    HINSTANCE(module.0)
}

/// UI 线程主体：建候选窗口后把 id 报回主线程，再跑消息循环，把命令应用到窗口。
fn run(commands: Receiver<UiCommand>, ready: &Sender<Option<u32>>) {
    // 按物理像素定位候选窗口，与应用（多为 per-monitor DPI 感知）报来的组句屏幕矩形对齐；缩放屏上不偏。
    // 进程级、只需设一次；已设过会返回失败，忽略。
    // SAFETY: 常量上下文，无输出参数。
    let _ = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    // SAFETY: 无参系统调用。
    let thread_id = unsafe { GetCurrentThreadId() };
    // 建窗口（顺带建起本线程的消息队列，之后 PostThreadMessageW 才有处可投）；失败就报回去、退出。
    let window = match CandidateWindow::new() {
        Ok(window) => window,
        Err(error) => {
            tracing::error!(%error, "建候选窗口失败，Server 将不显示候选框");
            let _ = ready.send(None);
            return;
        }
    };
    if ready.send(Some(thread_id)).is_err() {
        return;
    }
    let mut msg = MSG::default();
    loop {
        // SAFETY: msg 可写；取本线程全部消息。返回 0 = WM_QUIT，-1 = 出错，都退出。
        let got = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if got.0 <= 0 {
            break;
        }
        if msg.message == WM_WAKE {
            // 一次唤醒排空整个队列，按顺序应用（Hide→Show 这类先后关系要保住）。
            while let Ok(command) = commands.try_recv() {
                apply(&window, command);
            }
            continue;
        }
        // SAFETY: msg 有效。
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

fn apply(window: &CandidateWindow, command: UiCommand) {
    match command {
        UiCommand::Show(payload) => {
            let (frame, rect) = *payload;
            window.set_content(&frame);
            window.show(to_win_rect(rect));
        }
        UiCommand::Hide => window.hide(),
    }
}

fn to_win_rect(rect: ScreenRect) -> RECT {
    RECT {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    }
}
