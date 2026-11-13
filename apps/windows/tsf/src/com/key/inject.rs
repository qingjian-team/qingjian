//! 往输入队列里模拟方向键挪光标，给不接受输入法经 TSF 改选区的输入框用（见 [`caret_by_keys`]）。
//!
//! 敲左括号多半按着 Shift，直接发方向键应用会当成 Shift + 方向键去选字：按着的 Shift 先模拟抬起、发完方向键再模拟按回。
//! 模拟的 Shift 也会经过本输入法的击键 sink，单击 Shift 切中英的判定要跳过它们，所以返回模拟了几个 Shift 事件。

use std::sync::OnceLock;

use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT,
    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, SendInput, VIRTUAL_KEY, VK_LEFT, VK_LSHIFT, VK_RIGHT,
    VK_RSHIFT,
};
use windows::Win32::UI::TextServices::{ITfContext, TS_SS_TRANSITORY};
use windows::core::{s, w};

/// 这个输入框的光标要不要靠方向键挪（#972），两类：
/// - Chromium 系应用（[`chromium`]）；
/// - 系统代管的输入框（[`transitory`]），会话里改的选区会话结束就丢了。
pub(crate) fn caret_by_keys(context: &ITfContext) -> bool {
    chromium() || transitory(context)
}

/// 系统代管的输入框（上下文带 `TS_SS_TRANSITORY`）：Win32 Edit、RichEdit 2.0/3.0 这类不认 TSF 的老控件，
/// 由系统经 IMM 接到 TSF；TSF 里也读不到光标前后的文字。
pub(crate) fn transitory(context: &ITfContext) -> bool {
    unsafe { context.GetStatus() }.is_ok_and(|status| status.dwStaticFlags & TS_SS_TRANSITORY != 0)
}

/// 本进程是不是 Chromium 系（Chrome、Edge、Electron 应用如 VS Code / QQ、CEF 宿主如微信）。
/// 这些应用的文本存储在输入法是 IME 时不把它改的选区推给页面（Chromium `TSFTextStore::RequestLock` 的 `IsInputIME`）。
/// 认法：主程序导出 `GetHandleVerifier`（Chromium 的 exe 与 Electron 都有），或加载了 `libcef.dll`。
fn chromium() -> bool {
    static CHROMIUM: OnceLock<bool> = OnceLock::new();
    *CHROMIUM.get_or_init(|| unsafe {
        let exe = GetModuleHandleW(None)
            .ok()
            .and_then(|module| GetProcAddress(module, s!("GetHandleVerifier")))
            .is_some();
        exe || GetModuleHandleW(w!("libcef.dll")).is_ok()
    })
}

/// 发 `times` 下左（`right` 为假）或右方向键；返回模拟了几个 Shift 事件（击键 sink 要跳过的个数）。
pub(crate) fn press_arrow(right: bool, times: u32) -> u32 {
    let held: Vec<VIRTUAL_KEY> = [VK_LSHIFT, VK_RSHIFT]
        .into_iter()
        .filter(|key| unsafe { GetAsyncKeyState(i32::from(key.0)) } < 0)
        .collect();
    let arrow = if right { VK_RIGHT } else { VK_LEFT };
    let mut inputs = Vec::new();
    inputs.extend(held.iter().map(|key| input(*key, KEYEVENTF_KEYUP)));
    for _ in 0..times {
        inputs.push(input(arrow, KEYEVENTF_EXTENDEDKEY));
        inputs.push(input(arrow, KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP));
    }
    inputs.extend(held.iter().map(|key| input(*key, KEYBD_EVENT_FLAGS(0))));
    let sent = unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) } as usize;
    shift_events_sent(sent, inputs.len(), held.len())
}

/// 一共 `total` 个事件、头尾各 `held` 个 Shift，发出去了前 `sent` 个（被拦下时 `SendInput` 只发一部分）：其中有几个 Shift。
fn shift_events_sent(sent: usize, total: usize, held: usize) -> u32 {
    let tail = sent.saturating_sub(total - held);
    (sent.min(held) + tail) as u32
}

fn input(key: VIRTUAL_KEY, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                dwFlags: flags,
                ..Default::default()
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::shift_events_sent;

    #[test]
    fn counts_only_the_shift_events_that_went_out() {
        // 按着一个 Shift、退一格：抬起 Shift、方向键按下抬起、按回 Shift，共 4 个
        assert_eq!(shift_events_sent(4, 4, 1), 2);
        assert_eq!(shift_events_sent(0, 4, 1), 0);
        assert_eq!(shift_events_sent(1, 4, 1), 1);
        assert_eq!(shift_events_sent(3, 4, 1), 1);
        // 没按 Shift
        assert_eq!(shift_events_sent(2, 2, 0), 0);
    }
}
