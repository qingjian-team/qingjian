//! 合成用的位图：一张 top-down 32bpp DIB 与内存 DC。

use windows::Win32::Foundation::E_FAIL;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS,
    DeleteDC, DeleteObject, GdiFlush, HBITMAP, HDC, HGDIOBJ, SelectObject,
};
use windows::core::{Error, Result};

/// 一张 top-down 32bpp DIB 与选进了它的内存 DC。像素只能经 [`Self::pixels`]（`&mut self`）访问，
/// 保证 GDI 在画的时候没有 Rust 侧切片指着同一块内存。
pub(super) struct Canvas {
    dib: HBITMAP,
    memdc: HDC,
    previous: HGDIOBJ,
    bits: *mut u8,
    len: usize,
}

impl Canvas {
    pub(super) fn new(w: i32, h: i32) -> Result<Self> {
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

    pub(super) fn dc(&self) -> HDC {
        self.memdc
    }

    /// 像素（BGRA，top-down）。先 `GdiFlush` 把批量的 GDI 调用落到位图上。
    pub(super) fn pixels(&mut self) -> &mut [u8] {
        // SAFETY: 缓冲由 CreateDIBSection 分配、w*h*4 字节连续、随 dib 存活；`&mut self` 保证独占。
        unsafe {
            let _ = GdiFlush();
            std::slice::from_raw_parts_mut(self.bits, self.len)
        }
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.memdc, self.previous);
            let _ = DeleteDC(self.memdc);
            let _ = DeleteObject(self.dib.into());
        }
    }
}
