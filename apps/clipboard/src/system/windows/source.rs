//! 这次复制来自哪个程序，以及程序的图标。

use std::mem::size_of;

use bamboo_slip::tiny_skia::{ColorU8, Pixmap};
use windows::Win32::Foundation::{CloseHandle, HWND};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, DeleteObject, GetDC, GetDIBits,
    GetObjectW, HGDIOBJ, ReleaseDC,
};
use windows::Win32::Storage::FileSystem::{FILE_FLAGS_AND_ATTRIBUTES, GetDriveTypeW};
use windows::Win32::System::DataExchange::GetClipboardOwner;
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::Shell::{SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGetFileInfoW};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, GetForegroundWindow, GetIconInfo, GetWindowThreadProcessId, ICONINFO,
};
use windows::core::{HSTRING, PCWSTR, PWSTR};

/// 复制这段内容的程序路径；剪贴板没有主人时看前台窗口，都不知道时为空。
pub fn clipboard_source() -> String {
    let owner = unsafe { GetClipboardOwner() }.ok();
    owner
        .and_then(window_process_path)
        .or_else(|| window_process_path(unsafe { GetForegroundWindow() }))
        .unwrap_or_default()
}

fn window_process_path(window: HWND) -> Option<String> {
    if window.is_invalid() {
        return None;
    }
    unsafe {
        let mut process_id = 0;
        GetWindowThreadProcessId(window, Some(&mut process_id));
        if process_id == 0 {
            return None;
        }
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()?;
        let mut buffer = [0u16; 1024];
        let mut length = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        );
        let _ = CloseHandle(process);
        result.ok()?;
        Some(String::from_utf16_lossy(&buffer[..length as usize]))
    }
}

/// 程序的大图标；取不到为 `None`。
pub fn load_icon(path: &str) -> Option<Pixmap> {
    let wide = HSTRING::from(path);
    unsafe {
        let mut info = SHFILEINFOW::default();
        let found = SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut info),
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        );
        if found == 0 || info.hIcon.is_invalid() {
            return None;
        }
        let mut icon = ICONINFO::default();
        let read = GetIconInfo(info.hIcon, &mut icon);
        let _ = DestroyIcon(info.hIcon);
        read.ok()?;
        let pixmap = color_bitmap(&icon);
        let _ = DeleteObject(HGDIOBJ(icon.hbmColor.0));
        let _ = DeleteObject(HGDIOBJ(icon.hbmMask.0));
        pixmap
    }
}

unsafe fn color_bitmap(icon: &ICONINFO) -> Option<Pixmap> {
    unsafe {
        let mut bitmap = BITMAP::default();
        let written = GetObjectW(
            HGDIOBJ(icon.hbmColor.0),
            size_of::<BITMAP>() as i32,
            Some((&raw mut bitmap).cast()),
        );
        if written == 0 || bitmap.bmWidth <= 0 || bitmap.bmHeight <= 0 {
            return None;
        }
        let (width, height) = (bitmap.bmWidth as u32, bitmap.bmHeight as u32);
        let mut bgra = vec![0u8; (width * height * 4) as usize];
        let mut description = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                // 负数表示从上到下
                biHeight: -(height as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..BITMAPINFOHEADER::default()
            },
            ..BITMAPINFO::default()
        };
        let screen = GetDC(None);
        let lines = GetDIBits(
            screen,
            icon.hbmColor,
            0,
            height,
            Some(bgra.as_mut_ptr().cast()),
            &mut description,
            DIB_RGB_COLORS,
        );
        ReleaseDC(None, screen);
        if lines == 0 {
            return None;
        }
        // 老式图标没有 alpha 通道，全是 0：当作不透明
        let opaque = bgra.chunks_exact(4).all(|pixel| pixel[3] == 0);
        let mut pixmap = Pixmap::new(width, height)?;
        for (out, pixel) in pixmap.pixels_mut().iter_mut().zip(bgra.chunks_exact(4)) {
            let alpha = if opaque { 255 } else { pixel[3] };
            *out = ColorU8::from_rgba(pixel[2], pixel[1], pixel[0], alpha).premultiply();
        }
        Some(pixmap)
    }
}

/// `GetDriveTypeW` 的返回值：可移动盘、固定盘。
const DRIVE_REMOVABLE: u32 = 2;

const DRIVE_FIXED: u32 = 3;

/// 算总大小最多看多少个文件。
const MAX_SIZED_FILES: usize = 1000;

/// 这些文件一共多大。只看本机磁盘上的文件：网络盘查一下可能卡住监听线程，文件夹要递归，都按不知道算。
pub fn total_size(paths: &[String]) -> Option<u64> {
    if paths.len() > MAX_SIZED_FILES {
        return None;
    }
    let mut total = 0_u64;
    for path in paths {
        if !on_local_disk(path) {
            return None;
        }
        let metadata = std::fs::metadata(path).ok()?;
        if !metadata.is_file() {
            return None;
        }
        total = total.saturating_add(metadata.len());
    }
    Some(total)
}

fn on_local_disk(path: &str) -> bool {
    let root: Vec<u16> = path
        .chars()
        .take(3)
        .collect::<String>()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    if path.as_bytes().get(1..3).is_none_or(|part| part != b":\\") {
        return false;
    }
    // SAFETY: `root` 以 0 结尾，只在这次调用里读。
    matches!(
        unsafe { GetDriveTypeW(PCWSTR(root.as_ptr())) },
        DRIVE_FIXED | DRIVE_REMOVABLE
    )
}
