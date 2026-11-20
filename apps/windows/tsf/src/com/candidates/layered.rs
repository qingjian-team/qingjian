//! 分层窗口贴图：一张 top-down 32bpp DIB 与 `UpdateLayeredWindow`。
//!
//! 与 Server 进程里那张分层窗走同一套像素格式：渲染器出的位图是预乘 RGBA，DIB 是预乘 BGRA，
//! 只换通道顺序。窗口样式（`WS_EX_LAYERED | WS_EX_NOACTIVATE`）在 [`super`] 里建窗时给。

use qingjian_render::Pixmap;
use windows::Win32::Foundation::{COLORREF, E_FAIL, E_INVALIDARG, HWND, POINT, SIZE};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, HBITMAP, HDC,
    HGDIOBJ, SelectObject,
};
use windows::Win32::UI::WindowsAndMessaging::{ULW_ALPHA, UpdateLayeredWindow};
use windows::core::{Error, Result};

/// 一张 top-down 32bpp DIB 与选进了它的内存 DC。像素只在 [`Self::pixels`] 里借用，
/// 保证写像素时没有别的 Rust 切片指着同一块内存。
struct Canvas {
    dib: HBITMAP,
    memdc: HDC,
    previous: HGDIOBJ,
    bits: *mut u8,
    len: usize,
}

impl Canvas {
    fn new(w: i32, h: i32) -> Result<Self> {
        let header = BITMAPINFOHEADER {
            biSize: core::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // 负高 = top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        };
        let bmi = BITMAPINFO {
            bmiHeader: header,
            ..Default::default()
        };
        let mut bits = core::ptr::null_mut();
        let dib = unsafe { CreateDIBSection(None, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)? };
        let (memdc, previous) = unsafe {
            let memdc = CreateCompatibleDC(None);
            (memdc, SelectObject(memdc, dib.into()))
        };
        if bits.is_null() || memdc.is_invalid() {
            unsafe {
                let _ = DeleteDC(memdc);
                let _ = DeleteObject(dib.into());
            }
            return Err(Error::from(E_FAIL));
        }
        Ok(Self {
            dib,
            memdc,
            previous,
            bits: bits.cast(),
            len: (w * h * 4) as usize,
        })
    }

    fn dc(&self) -> HDC {
        self.memdc
    }

    /// 写像素：仅在把渲染器位图拷进 DIB 时用。
    fn pixels(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.bits, self.len) }
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        unsafe {
            let _ = SelectObject(self.memdc, self.previous);
            let _ = DeleteObject(self.dib.into());
            let _ = DeleteDC(self.memdc);
        }
    }
}

/// 把渲染器出的预乘 RGBA 位图（已含阴影边）贴到分层窗口上，`win_pos` 是位图左上角的屏幕坐标。
pub(super) fn present(hwnd: HWND, pixmap: &Pixmap, win_pos: (i32, i32)) -> Result<()> {
    let (w, h) = (pixmap.width() as i32, pixmap.height() as i32);
    if w <= 0 || h <= 0 {
        return Err(Error::from(E_INVALIDARG));
    }
    let mut canvas = Canvas::new(w, h)?;
    // tiny-skia 是 RGBA，DIB 是 BGRA；都是预乘，只换通道顺序。
    for (dst, src) in canvas.pixels().chunks_exact_mut(4).zip(pixmap.pixels()) {
        dst[0] = src.blue();
        dst[1] = src.green();
        dst[2] = src.red();
        dst[3] = src.alpha();
    }
    let dst = POINT {
        x: win_pos.0,
        y: win_pos.1,
    };
    let size = SIZE { cx: w, cy: h };
    let src = POINT { x: 0, y: 0 };
    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: AC_SRC_ALPHA as u8,
    };
    unsafe {
        UpdateLayeredWindow(
            hwnd,
            None,
            Some(&dst),
            Some(&size),
            Some(canvas.dc()),
            Some(&src),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        )
    }
}
