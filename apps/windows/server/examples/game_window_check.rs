//! 自己的无边框置顶窗口验证：无光标矩形也显示候选，显示不抢焦点，提交后收起。
//! 不启动或操作游戏，不读取用户配置或学习数据，不请求云服务。

use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use qingjian_platform::CandidateRenderer;
use qingjian_platform::protocol::{
    ClientMessage, KeyEvent, KeyModifiers, PROTOCOL_VERSION, SessionId,
};
use qingjian_windows_server::ui::UiHandle;
use qingjian_windows_server::{Router, RouterConfig};
use std::thread::sleep;
use std::time::Duration;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, FindWindowExW, GetForegroundWindow,
    GetWindowRect, GetWindowThreadProcessId, IsWindowVisible, RegisterClassW, SW_SHOW,
    SetForegroundWindow, ShowWindow, WNDCLASSW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::w;

fn candidate() -> Option<HWND> {
    let mut previous = None;
    loop {
        let hwnd =
            unsafe { FindWindowExW(None, previous, w!("QingjianCandidateWindow"), None) }.ok()?;
        if hwnd.is_invalid() {
            return None;
        }
        let mut pid = 0;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        if pid == unsafe { GetCurrentProcessId() } {
            return Some(hwnd);
        }
        previous = Some(hwnd);
    }
}

fn main() -> windows::core::Result<()> {
    let module = unsafe { GetModuleHandleW(None) }?;
    let class = WNDCLASSW {
        lpfnWndProc: Some(mock_wndproc),
        hInstance: HINSTANCE(module.0),
        lpszClassName: w!("QingjianGameCandidateProbe"),
        ..Default::default()
    };
    unsafe { RegisterClassW(&class) };
    let mock = unsafe {
        CreateWindowExW(
            WS_EX_TOPMOST,
            class.lpszClassName,
            w!("青简游戏候选显示检查"),
            WS_POPUP,
            200,
            180,
            800,
            600,
            None,
            None,
            Some(HINSTANCE(module.0)),
            None,
        )
    }?;
    let result = check(mock);
    let _ = unsafe { DestroyWindow(mock) };
    result
}

unsafe extern "system" fn mock_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn check(mock: HWND) -> windows::core::Result<()> {
    let _ = unsafe { ShowWindow(mock, SW_SHOW) };
    let _ = unsafe { SetForegroundWindow(mock) };
    sleep(Duration::from_millis(150));
    assert_eq!(
        unsafe { GetForegroundWindow() },
        mock,
        "自己的验证窗口需要前台焦点"
    );
    let app = std::env::current_exe()
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let mut config = RouterConfig {
        game_mode: true,
        renderer: CandidateRenderer::System,
        ..Default::default()
    };
    config.apps.game_candidates = vec![app.clone()];
    let mut router = Router::new(
        Engine::new(Dictionary::parse("你好\tni hao\t100\n").unwrap()),
        config,
    );
    let ui = UiHandle::spawn(Box::new(|_| {}), Box::new(|_| {}))?;
    router.set_candidate_sink(Box::new(ui));
    router.handle(ClientMessage::OpenSession {
        session: SessionId(1),
        app: Some(app),
        protocol: PROTOCOL_VERSION,
    });
    for c in "nihao".chars() {
        router.handle(ClientMessage::Key {
            session: SessionId(1),
            event: KeyEvent::new(c as u32, Some(c), KeyModifiers::default()),
        });
    }
    sleep(Duration::from_millis(300));
    let candidate = candidate().expect("自己的候选窗口");
    assert!(
        unsafe { IsWindowVisible(candidate) }.as_bool(),
        "无光标位置也必须显示候选"
    );
    assert_eq!(unsafe { GetForegroundWindow() }, mock, "候选不能抢焦点");
    let mut bounds = RECT::default();
    unsafe { GetWindowRect(candidate, &mut bounds) }?;
    assert!(
        bounds.left >= 200 && bounds.right <= 1000 && bounds.top >= 180 && bounds.bottom <= 780
    );
    router.handle(ClientMessage::Key {
        session: SessionId(1),
        event: KeyEvent::new(32, Some(' '), KeyModifiers::default()),
    });
    sleep(Duration::from_millis(150));
    assert!(
        !unsafe { IsWindowVisible(candidate) }.as_bool(),
        "提交后收起候选"
    );
    println!(
        "game_window_check=true no_caret_visible=true focus_preserved=true bounds_valid=true commit_hides=true"
    );
    Ok(())
}
