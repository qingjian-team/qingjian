//! 任务栏「中 / 英」图标的右键菜单。做法同小狼毫：`TrackPopupMenuEx` 挂在输入框所在窗口上，同步取回点的项。

use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, MENU_ITEM_FLAGS, MF_CHECKED, MF_GRAYED,
    MF_SEPARATOR, MF_STRING, TPM_BOTTOMALIGN, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    TrackPopupMenuEx,
};
use windows::core::{HSTRING, PCWSTR};

use qingjian_platform::protocol::IndicatorCommand;

/// 打开菜单时的勾选状态。
pub(crate) struct MenuState {
    pub(crate) english: bool,
    pub(crate) english_enabled: bool,
    pub(crate) full_width: bool,
    pub(crate) status_bar: bool,
    pub(crate) update_available: bool,
}

/// 点了哪一项：中 / 英 DLL 自己切，其余交给 Server。
pub(crate) enum MenuChoice {
    Mode { english: bool },
    Server(IndicatorCommand),
}

const ID_CHINESE: u32 = 1;
const ID_ENGLISH: u32 = 2;
const ID_PUNCTUATION: u32 = 3;
const ID_STATUS_BAR: u32 = 4;
const ID_SETTINGS: u32 = 5;
const ID_DOWNLOAD: u32 = 6;
const ID_SYMBOL_PANEL: u32 = 7;
const ID_LEGACY_SETTINGS: u32 = 8;

/// 在 `point`（屏幕坐标）弹出菜单，阻塞到用户点了某项或点别处关掉。
pub(crate) fn track(owner: HWND, point: POINT, state: &MenuState) -> Option<MenuChoice> {
    let checked = |on: bool| if on { MF_CHECKED } else { MENU_ITEM_FLAGS(0) };
    let english_flags = if state.english_enabled {
        checked(state.english)
    } else {
        MF_GRAYED
    };
    let mut items = vec![
        Some((ID_CHINESE, "中文", checked(!state.english))),
        Some((ID_ENGLISH, "英文", english_flags)),
        None,
        Some((ID_PUNCTUATION, "全角标点", checked(state.full_width))),
        Some((ID_STATUS_BAR, "悬浮状态条", checked(state.status_bar))),
        None,
        Some((ID_SYMBOL_PANEL, "符号面板…", MENU_ITEM_FLAGS(0))),
    ];
    if state.update_available {
        items.push(Some((
            ID_DOWNLOAD,
            "有新版本，前往下载…",
            MENU_ITEM_FLAGS(0),
        )));
    }
    items.push(Some((ID_SETTINGS, "设置…", MENU_ITEM_FLAGS(0))));
    items.push(Some((ID_LEGACY_SETTINGS, "旧版设置…", MENU_ITEM_FLAGS(0))));
    let menu = unsafe { CreatePopupMenu() }.ok()?;
    for item in items {
        let _ = match item {
            Some((id, text, flags)) => unsafe {
                AppendMenuW(menu, MF_STRING | flags, id as usize, &HSTRING::from(text))
            },
            None => unsafe { AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null()) },
        };
    }
    // 图标在任务栏上，菜单往上弹
    let flags = TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN;
    let id = unsafe { TrackPopupMenuEx(menu, flags.0, point.x, point.y, owner, None) };
    let _ = unsafe { DestroyMenu(menu) };
    match id.0 as u32 {
        ID_CHINESE => Some(MenuChoice::Mode { english: false }),
        ID_ENGLISH => Some(MenuChoice::Mode { english: true }),
        ID_PUNCTUATION => Some(MenuChoice::Server(IndicatorCommand::TogglePunctuation)),
        ID_STATUS_BAR => Some(MenuChoice::Server(IndicatorCommand::ToggleStatusBar)),
        ID_SETTINGS => Some(MenuChoice::Server(IndicatorCommand::OpenSettings)),
        ID_LEGACY_SETTINGS => Some(MenuChoice::Server(IndicatorCommand::OpenLegacySettings)),
        ID_DOWNLOAD => Some(MenuChoice::Server(IndicatorCommand::OpenDownload)),
        ID_SYMBOL_PANEL => Some(MenuChoice::Server(IndicatorCommand::OpenSymbolPanel)),
        _ => None,
    }
}
