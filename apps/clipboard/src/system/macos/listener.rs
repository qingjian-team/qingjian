//! 监听：后台线程按 `changeCount` 轮询剪贴板，读好内容后送给面板；主线程上每秒看一次配置文件，快捷键换了重新注册。
//! 全局快捷键与菜单栏图标的回调也在主线程，从这里把信号送出去。

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime};

use bamboo_slip::Waker;
use block2::RcBlock;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSWindowCollectionBehavior, NSWorkspace};
use objc2_foundation::NSTimer;
use qingjian_clipboard::ClipSettings;

use super::keycode::key_code;
use super::menu_state::MenuState;
use super::screen::{cursor_screen, panel_window};
use super::{hotkey, pasteboard, status_item};
use crate::system::Signal;

static OUTBOX: OnceLock<(Sender<Signal>, Waker)> = OnceLock::new();

/// 配置与它的修改时刻：主线程改，轮询线程读（要不要记这个程序）。
static STATE: Mutex<Option<(Watch, Option<SystemTime>)>> = Mutex::new(None);

/// 多久看一次剪贴板：系统没有变化通知，轮询只比一个整数，很便宜。
const POLL_INTERVAL: Duration = Duration::from_millis(300);

/// 多久看一次配置文件。
const WATCH_INTERVAL: f64 = 1.0;

/// 配置文件与它现在的样子。
pub struct Watch {
    /// `None` 时不看（命令行指定了数据、没有配置文件这类），一直用 `settings`。
    pub path: Option<PathBuf>,

    pub settings: ClipSettings,

    /// 由输入法拉起的：配置里关掉剪贴板就退出。
    pub managed: bool,
}

thread_local! {
    /// 看配置文件的定时器，留着它才不会被释放。
    static TIMER: RefCell<Option<Retained<NSTimer>>> = const { RefCell::new(None) };
}

fn modified(path: Option<&PathBuf>) -> Option<SystemTime> {
    std::fs::metadata(path?)
        .and_then(|meta| meta.modified())
        .ok()
}

fn with_state<T>(read: impl FnOnce(Option<&mut (Watch, Option<SystemTime>)>) -> T) -> T {
    let mut state = STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    read(state.as_mut())
}

/// 窗口宿主开窗前在主线程调。
pub fn spawn_listener(watch: Watch, sender: Sender<Signal>, waker: Waker) {
    if OUTBOX.set((sender, waker)).is_err() {
        return;
    }
    let Some(mtm) = MainThreadMarker::new() else {
        tracing::error!("clipboard listener must start on the main thread");
        return;
    };
    hotkey::register(watch.settings.hotkey);
    status_item::install(mtm);
    let watching = watch.path.is_some();
    let stamp = modified(watch.path.as_ref());
    *STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some((watch, stamp));
    if watching {
        let block = RcBlock::new(|_timer: std::ptr::NonNull<NSTimer>| check_config());
        // SAFETY: 定时器挂在主线程的事件循环上，块只在主线程跑。
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_repeats_block(WATCH_INTERVAL, true, &block)
        };
        TIMER.set(Some(timer));
    }
    let spawned = std::thread::Builder::new()
        .name("clipboard-listener".to_owned())
        .spawn(poll);
    if let Err(error) = spawned {
        tracing::error!(%error, "cannot start clipboard listener");
    }
}

fn poll() {
    let mut last = pasteboard::change_count();
    loop {
        std::thread::sleep(POLL_INTERVAL);
        let now = pasteboard::change_count();
        if now == last {
            continue;
        }
        last = now;
        let capture = pasteboard::read_capture(now, |source| {
            with_state(|state| state.is_none_or(|(watch, _)| watch.settings.records(source)))
        });
        if let Some(capture) = capture {
            send(Signal::Copied(capture));
        }
    }
}

/// 配置文件变了就读：关了退出，快捷键换了重新注册，别的交给面板。只在主线程调。
fn check_config() {
    let changed = with_state(|state| {
        let (watch, stamp) = state?;
        let now = modified(watch.path.as_ref());
        if now == *stamp {
            return None;
        }
        // 读成功才记下时刻：别的程序正写着时读不了、读到一半，下一秒要再读，不能当成看过了。
        // 写到一半的空文件也一样，按缺省算就成了「没开启」，进程会自己退出
        let text = std::fs::read_to_string(watch.path.as_ref()?).ok()?;
        if text.trim().is_empty() {
            return None;
        }
        let next = ClipSettings::parse(&text)?;
        *stamp = now;
        if watch.managed && !next.enabled {
            std::process::exit(0);
        }
        let previous = std::mem::replace(&mut watch.settings, next.clone());
        Some((previous, next))
    });
    let Some((previous, next)) = changed else {
        return;
    };
    if previous.hotkey != next.hotkey {
        hotkey::register(next.hotkey);
    }
    send(Signal::Settings(next));
}

fn send(signal: Signal) {
    if let Some((sender, waker)) = OUTBOX.get()
        && sender.send(signal).is_ok()
    {
        waker.wake();
    }
}

/// 呼出面板：粘贴回现在的前台程序。面板要能盖在全屏程序上、跟着用户切到的桌面走。
fn open_panel(mtm: MainThreadMarker, held_key: Option<u32>) {
    if let Some(window) = panel_window(mtm) {
        window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
    }
    let target = NSWorkspace::sharedWorkspace()
        .frontmostApplication()
        .map_or(0, |app| app.processIdentifier() as isize);
    let (area, scale) = cursor_screen(mtm);
    send(Signal::Hotkey {
        target,
        area,
        scale,
        held_key,
    });
}

/// 按了呼出快捷键（Carbon 在主线程回调）。
pub fn hotkey_pressed() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let held_key = with_state(|state| {
        state.and_then(|(watch, _)| key_code(watch.settings.hotkey.key).map(u32::from))
    });
    open_panel(mtm, held_key);
}

/// 菜单栏菜单里点了「打开面板」：菜单不抢前台，前台还是原来的程序。
pub fn open_from_menu(mtm: MainThreadMarker) {
    open_panel(mtm, None);
}

/// 菜单上显示的状态。
pub fn menu_state() -> MenuState {
    let (paused, managed) = with_state(|state| {
        state.map_or((false, false), |(watch, _)| {
            (watch.settings.paused, watch.managed)
        })
    });
    MenuState {
        paused,
        has_settings: settings_app().is_some(),
        standalone: !managed,
    }
}

/// 菜单改配置：写完马上读回来，菜单下次打开就是新状态。
fn write_flag(key: &str, value: bool) {
    let path = with_state(|state| state.and_then(|(watch, _)| watch.path.clone()));
    let Some(path) = path else {
        return;
    };
    match ClipSettings::write_flag(&path, key, value) {
        Ok(()) => check_config(),
        Err(error) => tracing::warn!(%error, key, "cannot write clipboard setting"),
    }
}

pub fn toggle_pause() {
    write_flag("paused", !menu_state().paused);
}

/// 随输入法装在同一个 `Contents/Applications/` 里的设置程序。
fn settings_app() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    // …/Contents/Applications/QingjianClipboard.app/Contents/MacOS/qingjian-clipboard
    let applications = exe.ancestors().nth(4)?;
    let app = applications.join("QingjianSettings.app");
    app.is_dir().then_some(app)
}

pub fn open_settings() {
    let Some(app) = settings_app() else {
        return;
    };
    if let Err(error) = std::process::Command::new("open").arg(app).spawn() {
        tracing::warn!(%error, "cannot open settings");
    }
}

/// 由输入法拉起的：退出就是关掉，不然输入法马上又把它拉起来，设置里的开关也对不上。
pub fn quit() {
    let (path, managed) = with_state(|state| {
        state.map_or((None, false), |(watch, _)| {
            (watch.path.clone(), watch.managed)
        })
    });
    if managed
        && let Some(path) = path
        && let Err(error) = ClipSettings::write_flag(&path, "enabled", false)
    {
        tracing::warn!(%error, "cannot turn the clipboard off");
    }
    std::process::exit(0);
}
