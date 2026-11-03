//! 系统剪贴板的读写：HTML、RTF、文件列表按原始字节存，写回去时一字不差，粘贴才能保留格式。

use std::borrow::Cow;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::time::Duration;

use qingjian_clipboard::{
    Capture, ClipFormat, ClipKind, FORMAT_FILES, FORMAT_HTML, FORMAT_PNG, FORMAT_RTF,
    MAX_FILE_LIST_BYTES, MAX_FORMAT_BYTES, MAX_TEXT_CHARS,
};
use windows::Win32::Foundation::{HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    RegisterClipboardFormatW, SetClipboardData,
};
use windows::Win32::System::Memory::{
    GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock,
};
use windows::core::{PCWSTR, w};

use super::source::{clipboard_source, total_size};
use crate::picture;
use crate::system::Payload;

const CF_UNICODETEXT: u32 = 13;

const CF_HDROP: u32 = 15;

const CF_DIB: u32 = 8;

/// 未压缩的图最多多大（字节）：再大的连读都不读，取出来就要占这么多内存。
const MAX_RAW_IMAGE_BYTES: usize = 200 * 1024 * 1024;

/// UTF-16 一个字符最多 4 字节。
const MAX_TEXT_BYTES: usize = MAX_TEXT_CHARS * 4;

/// 密码管理器等标记「别记录」用的剪贴板格式。
const SENSITIVE_FORMATS: [PCWSTR; 2] = [
    w!("ExcludeClipboardContentFromMonitorProcessing"),
    w!("Clipboard Viewer Ignore"),
];

/// 别的程序可能正占着剪贴板，打不开时重试这么多次。
const OPEN_ATTEMPTS: u32 = 10;

const OPEN_RETRY: Duration = Duration::from_millis(20);

/// 写完后等剪贴板变化的通知过去，这段时间内的通知是自己写的，不记。
const SELF_WRITE_SETTLE: Duration = Duration::from_millis(100);

static SELF_WRITE: AtomicBool = AtomicBool::new(false);

/// 监听线程的消息窗口：`SetClipboardData` 要求剪贴板有主人，否则系统可以让它失败。
static OWNER: AtomicIsize = AtomicIsize::new(0);

pub fn set_owner(window: HWND) {
    OWNER.store(window.0 as isize, Ordering::Relaxed);
}

/// 开着剪贴板的期间；离开作用域就关。
struct Opened;

impl Opened {
    fn new() -> Option<Self> {
        let owner = HWND(OWNER.load(Ordering::Relaxed) as *mut _);
        for attempt in 0..OPEN_ATTEMPTS {
            if unsafe { OpenClipboard(Some(owner)) }.is_ok() {
                return Some(Self);
            }
            if attempt + 1 < OPEN_ATTEMPTS {
                std::thread::sleep(OPEN_RETRY);
            }
        }
        tracing::warn!("cannot open the clipboard");
        None
    }

    /// 这种格式有多大（字节），没有为 0。
    fn size(&self, format: u32) -> usize {
        unsafe {
            GetClipboardData(format)
                .map(|handle| GlobalSize(HGLOBAL(handle.0)))
                .unwrap_or(0)
        }
    }

    /// 比 `limit` 大的不读，免得一次复制就占掉几百 MB。
    fn read(&self, format: u32, limit: usize) -> Option<Vec<u8>> {
        unsafe {
            let handle = GetClipboardData(format).ok()?;
            let global = HGLOBAL(handle.0);
            let size = GlobalSize(global);
            if size > limit {
                tracing::info!(format, size, "clipboard format too large to record");
                return None;
            }
            let pointer = GlobalLock(global);
            if pointer.is_null() {
                return None;
            }
            let bytes = std::slice::from_raw_parts(pointer.cast::<u8>(), size).to_vec();
            let _ = GlobalUnlock(global);
            Some(bytes)
        }
    }

    fn write(&self, format: u32, bytes: &[u8]) {
        unsafe {
            let Ok(global) = GlobalAlloc(GMEM_MOVEABLE, bytes.len()) else {
                tracing::warn!("clipboard allocation failed");
                return;
            };
            let pointer = GlobalLock(global);
            if pointer.is_null() {
                return;
            }
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer.cast::<u8>(), bytes.len());
            let _ = GlobalUnlock(global);
            // 成功后这块内存归系统
            if let Err(error) = SetClipboardData(format, Some(HANDLE(global.0))) {
                tracing::warn!(%error, format, "clipboard write failed");
            }
        }
    }
}

impl Drop for Opened {
    fn drop(&mut self) {
        let _ = unsafe { CloseClipboard() };
    }
}

fn html_format() -> u32 {
    unsafe { RegisterClipboardFormatW(w!("HTML Format")) }
}

fn rtf_format() -> u32 {
    unsafe { RegisterClipboardFormatW(w!("Rich Text Format")) }
}

fn sensitive() -> bool {
    SENSITIVE_FORMATS.iter().any(|&name| unsafe {
        let format = RegisterClipboardFormatW(name);
        format != 0 && IsClipboardFormatAvailable(format).is_ok()
    })
}

/// 读这次复制的内容。文件优先，其次文字（网页、Office 同时给文字和图片时用户多半要文字），最后才是图片。
/// `records` 按来源程序判断要不要记，不记的内容根本不读。
pub fn read_capture(records: impl Fn(&str) -> bool) -> Option<Capture> {
    if SELF_WRITE.load(Ordering::Relaxed) || sensitive() {
        return None;
    }
    let source = clipboard_source();
    if !records(&source) {
        return None;
    }
    let image_with_files;
    let (files, text, html, rtf) = {
        let opened = Opened::new()?;
        // 文件列表、文字超限时整条不记，不能往下落到别的格式上，记成另一样东西
        let has_files = opened.size(CF_HDROP) > 0;
        let image_size = opened.size(CF_DIB);
        // 有文件列表时图太大不算超限：退回只记路径
        if opened.size(CF_HDROP) > MAX_FILE_LIST_BYTES
            || opened.size(CF_UNICODETEXT) > MAX_TEXT_BYTES
            || (!has_files && image_size > MAX_RAW_IMAGE_BYTES)
        {
            tracing::info!("clipboard content too large to record");
            return None;
        }
        image_with_files = has_files && image_size > 0 && image_size <= MAX_RAW_IMAGE_BYTES;
        (
            opened.read(CF_HDROP, MAX_FILE_LIST_BYTES),
            opened.read(CF_UNICODETEXT, MAX_TEXT_BYTES),
            opened.read(html_format(), MAX_FORMAT_BYTES),
            opened.read(rtf_format(), MAX_FORMAT_BYTES),
        )
    };
    if let Some(raw) = files
        && let Some(paths) = parse_file_list(&raw)
    {
        if image_with_files && let Some((png, thumbnail)) = read_image() {
            return Some(Capture::image_with_files(png, thumbnail, raw, source));
        }
        let size = total_size(&paths);
        return Some(Capture::files(&paths, raw, source, size));
    }
    if let Some(text) = text.map(|bytes| decode_text(&bytes))
        && !text.trim().is_empty()
    {
        let mut formats = Vec::new();
        if let Some(data) = html {
            formats.push(ClipFormat::new(FORMAT_HTML, data));
        }
        if let Some(data) = rtf {
            formats.push(ClipFormat::new(FORMAT_RTF, data));
        }
        return Some(Capture::text(text, source, formats));
    }
    let (png, thumbnail) = read_image()?;
    Some(Capture::image(png, thumbnail, source))
}

fn read_image() -> Option<(Vec<u8>, Vec<u8>)> {
    let image = arboard::Clipboard::new().ok()?.get_image().ok()?;
    picture::encode(image.width as u32, image.height as u32, &image.bytes)
}

/// 写回系统剪贴板；返回时自己写引起的通知已经过去。
pub fn write_payload(payload: &Payload) {
    SELF_WRITE.store(true, Ordering::Relaxed);
    if payload.kind == ClipKind::Image {
        write_image(payload);
    } else if let Some(opened) = Opened::new()
        && unsafe { EmptyClipboard() }.is_ok()
    {
        if payload.kind != ClipKind::Files {
            opened.write(CF_UNICODETEXT, &encode_text(&payload.text));
        }
        for format in &payload.formats {
            let id = match format.name.as_str() {
                FORMAT_HTML => html_format(),
                FORMAT_RTF => rtf_format(),
                FORMAT_FILES => CF_HDROP,
                _ => continue,
            };
            opened.write(id, &format.data);
        }
    }
    std::thread::sleep(SELF_WRITE_SETTLE);
    SELF_WRITE.store(false, Ordering::Relaxed);
}

fn write_image(payload: &Payload) {
    let decoded = payload
        .formats
        .iter()
        .find(|format| format.name == FORMAT_PNG)
        .and_then(|format| picture::decode(&format.data));
    let Some((width, height, rgba)) = decoded else {
        tracing::warn!("clipboard image missing or unreadable");
        return;
    };
    let image = arboard::ImageData {
        width: width as usize,
        height: height as usize,
        bytes: Cow::Owned(rgba),
    };
    if let Err(error) = arboard::Clipboard::new().and_then(|mut board| board.set_image(image)) {
        tracing::warn!(%error, "clipboard image write failed");
        return;
    }
    // 文件还在才加上：目标程序可能优先读文件，读到失效的路径就什么都粘贴不了。这里不清空，图片留着
    let files = payload
        .formats
        .iter()
        .find(|format| format.name == FORMAT_FILES)
        .filter(|format| {
            parse_file_list(&format.data)
                .is_some_and(|paths| paths.iter().all(|path| Path::new(path).exists()))
        });
    if let Some(files) = files
        && let Some(opened) = Opened::new()
    {
        opened.write(CF_HDROP, &files.data);
    }
}

fn decode_text(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .take_while(|&unit| unit != 0)
        .collect();
    String::from_utf16_lossy(&units)
}

fn encode_text(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(u16::to_le_bytes)
        .collect()
}

/// 解 `DROPFILES`：头 20 字节，`pFiles` 在 0、`fWide` 在 16，之后是 NUL 分隔、双 NUL 结尾的路径。只认宽字符。
fn parse_file_list(raw: &[u8]) -> Option<Vec<String>> {
    let offset = u32::from_le_bytes(raw.get(0..4)?.try_into().ok()?) as usize;
    let wide = u32::from_le_bytes(raw.get(16..20)?.try_into().ok()?) != 0;
    if !wide {
        return None;
    }
    let units: Vec<u16> = raw
        .get(offset..)?
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    let paths: Vec<String> = units
        .split(|&unit| unit == 0)
        .take_while(|path| !path.is_empty())
        .map(String::from_utf16_lossy)
        .collect();
    (!paths.is_empty()).then_some(paths)
}

#[cfg(test)]
mod tests {
    use super::{decode_text, encode_text, parse_file_list};

    fn drop_files(paths: &[&str], wide: u32) -> Vec<u8> {
        let mut raw = vec![0u8; 20];
        raw[0] = 20;
        raw[16..20].copy_from_slice(&wide.to_le_bytes());
        for path in paths {
            raw.extend(encode_text(path));
        }
        raw.extend([0, 0]);
        raw
    }

    #[test]
    fn file_list_is_split_into_paths() {
        let raw = drop_files(&["C:\\a b.txt", "D:\\文件.png"], 1);
        assert_eq!(
            parse_file_list(&raw).unwrap(),
            ["C:\\a b.txt", "D:\\文件.png"]
        );
    }

    #[test]
    fn narrow_or_broken_file_lists_are_rejected() {
        assert!(parse_file_list(&drop_files(&["C:\\a"], 0)).is_none());
        assert!(parse_file_list(&[1, 2, 3]).is_none());
        assert!(parse_file_list(&drop_files(&[], 1)).is_none());
    }

    #[test]
    fn text_round_trips_and_stops_at_the_terminator() {
        let mut bytes = encode_text("中文\r\n  code");
        bytes.extend([65, 0]);
        assert_eq!(decode_text(&bytes), "中文\r\n  code");
    }
}
