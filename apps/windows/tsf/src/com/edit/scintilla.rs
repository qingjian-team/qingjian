//! Scintilla 的 IMM 兼容上下文读不到非组句选区；只在本进程焦点控件按原生消息补读。

use windows::Win32::Foundation::LPARAM;
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::Input::KeyboardAndMouse::GetFocus;
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetWindowThreadProcessId, SendMessageW,
};

const SCI_GETCODEPAGE: u32 = 2137;
const SCI_GETSELTEXT: u32 = 2161;
const SCI_SELECTIONISRECTANGLE: u32 = 2372;
const SCI_GETSELECTIONS: u32 = 2570;

pub(super) fn selected_text() -> Option<String> {
    let window = unsafe { GetFocus() };
    if window.is_invalid() {
        return None;
    }
    let mut process = 0;
    unsafe { GetWindowThreadProcessId(window, Some(&mut process)) };
    if process != unsafe { GetCurrentProcessId() } {
        return None;
    }
    let mut class = [0u16; 64];
    let count = unsafe { GetClassNameW(window, &mut class) } as usize;
    if String::from_utf16_lossy(&class[..count]) != "Scintilla" {
        return None;
    }
    let query = |message| unsafe { SendMessageW(window, message, None, None).0 };
    // 不猜非 UTF-8 编码，也不把矩形 / 多选区合并后写回成一个错误范围。
    if query(SCI_GETCODEPAGE) != 65001
        || query(SCI_GETSELECTIONS) != 1
        || query(SCI_SELECTIONISRECTANGLE) != 0
    {
        return None;
    }
    let bytes = usize::try_from(query(SCI_GETSELTEXT)).ok()?;
    if !(1..=2001).contains(&bytes) {
        return None;
    }
    let mut buffer = vec![0u8; bytes + 1];
    unsafe {
        SendMessageW(
            window,
            SCI_GETSELTEXT,
            None,
            Some(LPARAM(buffer.as_mut_ptr() as isize)),
        );
    }
    decode_selection(&buffer, bytes)
}

fn decode_selection(buffer: &[u8], bytes: usize) -> Option<String> {
    let end = buffer.iter().position(|&byte| byte == 0)?;
    if end + 1 < bytes {
        return None;
    }
    let text = std::str::from_utf8(&buffer[..end]).ok()?;
    (!text.is_empty() && text.encode_utf16().count() <= 500).then(|| text.to_owned())
}

#[cfg(test)]
mod tests {
    fn read(text: &str) -> Option<String> {
        let mut buffer = text.as_bytes().to_vec();
        buffer.push(0);
        super::decode_selection(&buffer, buffer.len())
    }

    #[test]
    fn selection_limit_counts_utf16_units_instead_of_utf8_bytes() {
        assert!(read(&"中".repeat(500)).is_some());
        assert!(read(&"中".repeat(501)).is_none());
        assert!(read(&"😀".repeat(250)).is_some());
        assert!(read(&"😀".repeat(251)).is_none());
    }

    #[test]
    fn malformed_or_empty_text_cannot_become_a_partial_translation() {
        assert!(read("").is_none());
        assert!(read("before\0after").is_none());
        assert!(super::decode_selection(&[0xff, 0], 2).is_none());
        assert!(super::decode_selection(b"unterminated", 12).is_none());
    }
}
