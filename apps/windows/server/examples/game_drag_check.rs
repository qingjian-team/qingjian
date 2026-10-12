//! 只操作测试进程自己的候选 HWND，验证拖动、不抢焦点、重绘与重建后的位置恢复。
//! 不向前台应用发送输入，不读取用户设置，不请求云服务；APPDATA 必须为隔离测试目录。

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
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT, WPARAM};
use windows::Win32::System::Threading::{
    GetCurrentProcessId, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    QueryFullProcessImageNameW,
};

use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId, HTCLIENT,
    IsWindowVisible, MA_NOACTIVATE, SendMessageW, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEACTIVATE,
    WM_MOUSEMOVE, WM_NCHITTEST,
};
use windows::core::{PWSTR, w};

fn visible_candidate() -> HWND {
    let mut previous = None;
    loop {
        let window =
            unsafe { FindWindowExW(None, previous, w!("QingjianCandidateWindow"), None) }.unwrap();
        assert!(!window.is_invalid(), "自己的可见候选窗口");
        let mut pid = 0;
        unsafe { GetWindowThreadProcessId(window, Some(&mut pid)) };
        if pid == unsafe { GetCurrentProcessId() } && unsafe { IsWindowVisible(window) }.as_bool() {
            return window;
        }
        previous = Some(window);
    }
}

fn bounds(window: HWND) -> RECT {
    let mut rect = RECT::default();
    unsafe { GetWindowRect(window, &mut rect) }.unwrap();
    rect
}

fn key(router: &mut Router, c: char) {
    router.handle(ClientMessage::Key {
        session: SessionId(1),
        event: KeyEvent::new(c as u32, Some(c), KeyModifiers::default()),
    });
}

fn mouse(window: HWND, message: u32, x: i32, y: i32) {
    let lparam = LPARAM(((x & 0xffff) | ((y & 0xffff) << 16)) as isize);
    unsafe { SendMessageW(window, message, Some(WPARAM(0)), Some(lparam)) };
}

fn main() {
    let foreground = unsafe { GetForegroundWindow() };
    assert!(!foreground.is_invalid());
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(foreground, Some(&mut pid)) };
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.unwrap();
    let mut path = [0_u16; 32768];
    let mut length = path.len() as u32;
    unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(path.as_mut_ptr()),
            &mut length,
        )
    }
    .unwrap();
    let _ = unsafe { CloseHandle(process) };
    let path = String::from_utf16_lossy(&path[..length as usize]);
    let app = path.rsplit('\\').next().unwrap().to_owned();
    let position_file = qingjian_platform::dirs::user_dir()
        .unwrap()
        .join("game-candidate-positions.toml");
    assert!(
        position_file
            .to_string_lossy()
            .contains("game-candidate-drag"),
        "必须隔离测试位置文件"
    );
    let _ = std::fs::remove_file(&position_file);
    let mut config = RouterConfig {
        game_mode: true,
        renderer: if std::env::args().any(|a| a == "--qingjian-renderer") {
            CandidateRenderer::Qingjian
        } else {
            CandidateRenderer::System
        },
        ..Default::default()
    };
    config.apps.game_candidates = vec![app.clone()];
    let mut router = Router::new(
        Engine::new(Dictionary::parse("你好\tni hao\t100\n").unwrap()),
        config,
    );
    router.set_candidate_sink(Box::new(
        UiHandle::spawn(Box::new(|_| {}), Box::new(|_| {})).unwrap(),
    ));
    router.handle(ClientMessage::OpenSession {
        session: SessionId(1),
        app: Some(app),
        protocol: PROTOCOL_VERSION,
    });
    for c in "nihao".chars() {
        key(&mut router, c);
    }
    sleep(Duration::from_millis(250));
    let window = visible_candidate();
    assert_eq!(
        unsafe { SendMessageW(window, WM_MOUSEACTIVATE, None, None) }.0,
        MA_NOACTIVATE as isize
    );
    let before = bounds(window);
    // 新主题的投影留白可变；通过实际命中测试找到顶部可拖区域，避免把投影当拼音栏。
    let x = (before.right - before.left) / 2;
    let y = (0..(before.bottom - before.top))
        .find(|y| {
            let sx = before.left + x;
            let sy = before.top + y;
            let point = LPARAM(((sx & 0xffff) | ((sy & 0xffff) << 16)) as isize);
            unsafe { SendMessageW(window, WM_NCHITTEST, None, Some(point)) }.0 == HTCLIENT as isize
        })
        .expect("游戏候选顶部必须可接受鼠标")
        + 2;
    mouse(window, WM_LBUTTONDOWN, x, y);
    mouse(window, WM_MOUSEMOVE, x + 60, y + 30);
    mouse(window, WM_LBUTTONUP, x, y);
    let moved = bounds(window);
    assert!(
        moved.left != before.left || moved.top != before.top,
        "拖动必须移动"
    );
    assert_eq!(
        unsafe { GetForegroundWindow() },
        foreground,
        "拖动不能抢前台焦点"
    );
    assert!(position_file.is_file(), "松手保存位置");
    key(&mut router, 'n');
    sleep(Duration::from_millis(200));
    let redrawn = bounds(visible_candidate());
    assert!(
        (redrawn.left - moved.left).abs() <= 3 && (redrawn.top - moved.top).abs() <= 3,
        "重绘不能跳回"
    );
    router.handle(ClientMessage::Commit {
        session: SessionId(1),
    });
    router.set_candidate_sink(Box::new(
        UiHandle::spawn(Box::new(|_| {}), Box::new(|_| {})).unwrap(),
    ));
    for c in "nihao".chars() {
        key(&mut router, c);
    }
    sleep(Duration::from_millis(200));
    let restored = bounds(visible_candidate());
    assert!(
        (restored.left - moved.left).abs() <= 3 && (restored.top - moved.top).abs() <= 3,
        "重建窗口应恢复位置"
    );
    router.handle(ClientMessage::Commit {
        session: SessionId(1),
    });
    println!(
        "game_drag_check=true drag_moves=true no_activation=true redraw_keeps_position=true recreated_window_restores=true no_input_sent=true"
    );
}
