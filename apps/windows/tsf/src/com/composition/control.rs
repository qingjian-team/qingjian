//! 直接问焦点上的 Win32 Edit / RichEdit 控件光标后是什么字。
//! 系统代管的输入框（`TS_SS_TRANSITORY`，见 `key::inject::caret_by_keys`）在 TSF 里读不到光标后的文字，
//! 成对符号跳过右半边前的核对改问控件本身；输入法跑在应用的界面线程里，消息直接送到窗口过程。

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Controls::EM_GETSEL;
use windows::Win32::UI::Controls::RichEdit::{CHARRANGE, EM_GETTEXTRANGE, TEXTRANGEW};
use windows::Win32::UI::Input::KeyboardAndMouse::GetFocus;
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetWindowTextLengthW, GetWindowTextW, SendMessageW,
};
use windows::core::PWSTR;

/// 焦点控件里插入点（选区终点）后的一个字；焦点不是 Edit / RichEdit、或在末尾时为 `None`。
/// 类名按包含判断，WinForms 这类超类化的控件（`WindowsForms10.EDIT…`、`WindowsForms10.RichEdit20W…`）也认。
pub(super) fn next_char() -> Option<char> {
    let window = unsafe { GetFocus() };
    if window.is_invalid() {
        return None;
    }
    let mut name = [0u16; 128];
    let length = unsafe { GetClassNameW(window, &mut name) };
    let class =
        String::from_utf16_lossy(&name[..usize::try_from(length).ok()?]).to_ascii_lowercase();
    let unit = if class.contains("richedit") {
        let end = caret(window);
        // RichEdit 的位置把换段算一个字，与 WM_GETTEXT 给的 \r\n 对不上，按位置取
        let mut buf = [0u16; 2];
        let mut range = TEXTRANGEW {
            chrg: CHARRANGE {
                cpMin: end as i32,
                cpMax: end as i32 + 1,
            },
            lpstrText: PWSTR(buf.as_mut_ptr()),
        };
        let got = unsafe {
            SendMessageW(
                window,
                EM_GETTEXTRANGE,
                None,
                Some(LPARAM(&raw mut range as isize)),
            )
        };
        (got.0 == 1).then_some(buf[0])?
    } else if class.contains("edit") {
        let end = caret(window);
        let total = unsafe { GetWindowTextLengthW(window) };
        let mut text = vec![0u16; usize::try_from(total).ok()? + 1];
        let copied = usize::try_from(unsafe { GetWindowTextW(window, &mut text) }).ok()?;
        *text[..copied].get(end as usize)?
    } else {
        return None;
    };
    char::from_u32(u32::from(unit))
}

/// 插入点：选区终点（EM_GETSEL 带指针的那种用法，超过 65535 也对）。
fn caret(window: HWND) -> u32 {
    let mut start = 0u32;
    let mut end = 0u32;
    unsafe {
        SendMessageW(
            window,
            EM_GETSEL,
            Some(WPARAM(std::ptr::from_mut(&mut start) as usize)),
            Some(LPARAM(std::ptr::from_mut(&mut end) as isize)),
        );
    }
    end
}
