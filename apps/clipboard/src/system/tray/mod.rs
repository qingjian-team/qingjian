//! 系统托盘：图标、右键菜单，以及单独运行时的开机启动。按仓库约定用系统原生控件，不走自绘。

mod choice;
mod menu_state;

use std::mem::size_of;

use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW,
    RegSetKeyValueW,
};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
    Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, GetSystemMetrics, HICON,
    IDI_APPLICATION, IMAGE_ICON, LR_SHARED, LoadIconW, LoadImageW, MENU_ITEM_FLAGS, MF_CHECKED,
    MF_SEPARATOR, MF_STRING, PostMessageW, SM_CXSMICON, SM_CYSMICON, SetForegroundWindow,
    TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenuEx, WM_APP, WM_NULL,
};
use windows::core::{HSTRING, PCWSTR, w};

use crate::i18n::tr;

pub use choice::Choice;
pub use menu_state::MenuState;

/// 托盘图标点了之后发给窗口的消息。
pub const CALLBACK: u32 = WM_APP + 1;

const ICON_ID: u32 = 1;

const AUTOSTART_KEY: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");

const AUTOSTART_VALUE: PCWSTR = w!("QingjianClipboard");

/// 嵌进 exe 的托盘图标资源 id，见 build.rs。
const ICON_ON_DARK: usize = 2;

const ICON_ON_LIGHT: usize = 3;

const PERSONALIZE_KEY: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");

const CHOICES: [Choice; 5] = [
    Choice::Open,
    Choice::TogglePause,
    Choice::OpenSettings,
    Choice::ToggleAutostart,
    Choice::Quit,
];

/// 菜单项的 id，从 1 起（0 是没选）。
fn id(choice: Choice) -> usize {
    CHOICES.iter().position(|item| *item == choice).unwrap_or(0) + 1
}

/// 任务栏是浅色的。任务栏跟的是系统模式，不是应用模式；读不到时按深色算，那是系统的缺省。
fn taskbar_is_light() -> bool {
    let mut value = 0_u32;
    let mut size = size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PERSONALIZE_KEY,
            w!("SystemUsesLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some((&raw mut value).cast()),
            Some(&mut size),
        )
    };
    status.is_ok() && value != 0
}

/// 按小图标的尺寸取：托盘用它，交给系统缩放会糊。共享的句柄不用释放；没嵌上时用系统的。
fn icon() -> HICON {
    let id = if taskbar_is_light() {
        ICON_ON_LIGHT
    } else {
        ICON_ON_DARK
    };
    let instance = unsafe { GetModuleHandleW(None) }.ok();
    unsafe {
        LoadImageW(
            instance.map(Into::into),
            PCWSTR(std::ptr::without_provenance(id)),
            IMAGE_ICON,
            GetSystemMetrics(SM_CXSMICON),
            GetSystemMetrics(SM_CYSMICON),
            LR_SHARED,
        )
    }
    .map(|handle| HICON(handle.0))
    .or_else(|_| unsafe { LoadIconW(None, IDI_APPLICATION) })
    .unwrap_or_default()
}

fn data(window: HWND) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: window,
        uID: ICON_ID,
        ..NOTIFYICONDATAW::default()
    }
}

/// 加图标；资源管理器重启后也要再调一次。
pub fn add(window: HWND) {
    let mut icon_data = data(window);
    icon_data.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    icon_data.uCallbackMessage = CALLBACK;
    icon_data.hIcon = icon();
    for (out, unit) in icon_data
        .szTip
        .iter_mut()
        .zip(tr("tray.tip").encode_utf16().take(127))
    {
        *out = unit;
    }
    if !unsafe { Shell_NotifyIconW(NIM_ADD, &icon_data) }.as_bool() {
        tracing::warn!("cannot add the tray icon");
    }
}

/// 系统换了深浅色：换一张图标。
pub fn refresh(window: HWND) {
    let mut icon_data = data(window);
    icon_data.uFlags = NIF_ICON;
    icon_data.hIcon = icon();
    if !unsafe { Shell_NotifyIconW(NIM_MODIFY, &icon_data) }.as_bool() {
        tracing::warn!("cannot update the tray icon");
    }
}

pub fn remove(window: HWND) {
    let _ = unsafe { Shell_NotifyIconW(NIM_DELETE, &data(window)) };
}

/// 弹出右键菜单，等用户选完。
pub fn show_menu(window: HWND, state: &MenuState) -> Option<Choice> {
    let menu = unsafe { CreatePopupMenu() }.ok()?;
    let check = |on: bool| -> MENU_ITEM_FLAGS {
        if on {
            MF_STRING | MF_CHECKED
        } else {
            MF_STRING
        }
    };
    let mut items = vec![
        (Choice::Open, MF_STRING, tr("tray.open")),
        (Choice::TogglePause, check(state.paused), tr("tray.pause")),
    ];
    if state.has_settings {
        items.push((Choice::OpenSettings, MF_STRING, tr("tray.settings")));
    }
    if state.standalone {
        items.push((
            Choice::ToggleAutostart,
            check(autostart_enabled()),
            tr("tray.autostart"),
        ));
    }
    unsafe {
        for (choice, flags, label) in items {
            let _ = AppendMenuW(menu, flags, id(choice), &HSTRING::from(label));
        }
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            id(Choice::Quit),
            &HSTRING::from(tr("tray.quit")),
        );
        let mut cursor = POINT::default();
        let _ = GetCursorPos(&mut cursor);
        // 不先拿前台，点到菜单外面它不会收起（系统文档里的要求）
        let _ = SetForegroundWindow(window);
        let picked = TrackPopupMenuEx(
            menu,
            (TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON).0,
            cursor.x,
            cursor.y,
            window,
            None,
        );
        let _ = PostMessageW(Some(window), WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);
        let picked = picked.0 as usize;
        (picked > 0)
            .then(|| CHOICES.get(picked - 1).copied())
            .flatten()
    }
}

pub fn autostart_enabled() -> bool {
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            AUTOSTART_KEY,
            AUTOSTART_VALUE,
            RRF_RT_REG_SZ,
            None,
            None,
            None,
        )
    };
    status.is_ok()
}

/// 开或关开机启动，只动自己这一个值。
pub fn set_autostart(on: bool) {
    let status = if on {
        let Ok(exe) = std::env::current_exe() else {
            return;
        };
        let command: Vec<u16> = format!("\"{}\"", exe.display())
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                AUTOSTART_KEY,
                AUTOSTART_VALUE,
                REG_SZ.0,
                Some(command.as_ptr().cast()),
                (command.len() * 2) as u32,
            )
        }
    } else {
        unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, AUTOSTART_KEY, AUTOSTART_VALUE) }
    };
    if status.is_err() {
        tracing::warn!(on, ?status, "cannot change autostart");
    }
}
