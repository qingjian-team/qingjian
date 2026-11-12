//! 往输入队列里模拟方向键挪光标，给不接受输入法经 TSF 改选区的应用用（Chromium 系，见 [`caret_by_keys`]）。
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
use windows::core::{s, w};

/// 本进程是不是 Chromium 系（Chrome、Edge、Electron 应用如 VS Code / QQ、CEF 宿主如微信）。
/// 这些应用的文本存储在输入法是 IME 时不把它改的选区推给页面（Chromium `TSFTextStore::RequestLock` 的 `IsInputIME`），
/// 光标只能靠方向键挪。认法：主程序导出 `GetHandleVerifier`（Chromium 的 exe 与 Electron 都有），或加载了 `libcef.dll`。
pub(crate) fn caret_by_keys() -> bool {
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
    // 只数真发出去的 Shift 事件：Shift 在队列两头，发了一部分时前面的抬起可能已经出去了
    let shifts = held.len();
    let tail = sent.saturating_sub(inputs.len() - shifts);
    (sent.min(shifts) + tail) as u32
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
