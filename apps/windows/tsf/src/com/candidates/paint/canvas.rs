//! 画布：一张 top-down 32bpp DIB 与选进它的内存 DC，画完直接贴给分层窗口。
//!
//! GDI 只写 RGB、不写 alpha，所以贴图前按圆角矩形给内外补 alpha：窗口内不透、窗口外全透明，
//! 圆角边缘按到弧线的距离给覆盖度，圆角才不是硬台阶。预乘 alpha 的位图要求 RGB 先乘上 alpha，
//! 这一步也在 [`Canvas::finish`] 里做。

use windows::Win32::Foundation::{COLORREF, E_FAIL, E_INVALIDARG, HWND, POINT, SIZE};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    CreateCompatibleDC, CreateDIBSection, CreateRoundRectRgn, CreateSolidBrush, DIB_RGB_COLORS,
    DeleteDC, DeleteObject, FillRgn, HBITMAP, HDC, HGDIOBJ, SelectObject,
};
use windows::Win32::UI::WindowsAndMessaging::{ULW_ALPHA, UpdateLayeredWindow};
use windows::core::{Error, Result};

/// 一张 top-down 32bpp DIB 与选进了它的内存 DC。像素只在 [`Self::finish`] 里借用，
/// 保证补 alpha 时没有别的 Rust 切片指着同一块内存。
pub(crate) struct Canvas {
    dib: HBITMAP,
    memdc: HDC,
    previous: HGDIOBJ,
    bits: *mut u8,
    width: i32,
    height: i32,
}

impl Canvas {
    pub(crate) fn new(width: i32, height: i32) -> Result<Self> {
        if width <= 0 || height <= 0 {
            return Err(Error::from(E_INVALIDARG));
        }
        let header = BITMAPINFOHEADER {
            biSize: core::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height, // 负高 = top-down
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
        let dib = unsafe { CreateDIBSection(None, &bmi, DIB_RGB_COLORS, &mut bits, None, 0) }?;
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
            width,
            height,
        })
    }

    pub(crate) fn dc(&self) -> HDC {
        self.memdc
    }

    /// 铺底：一圈描边套着背景色，圆角与窗口一致。
    pub(crate) fn fill_background(&mut self, background: COLORREF, border: COLORREF, radius: i32) {
        fill_round_rect(self.memdc, (0, 0, self.width, self.height), border, radius);
        if self.width > 2 && self.height > 2 {
            fill_round_rect(
                self.memdc,
                (1, 1, self.width - 1, self.height - 1),
                background,
                (radius - 1).max(0),
            );
        }
    }

    /// 圆角外置透明、圆角内置不透（GDI 不写 alpha，这一步补上）。
    pub(crate) fn finish(&mut self, radius: i32) {
        debug_assert!(!self.bits.is_null());
        let len = (self.width * self.height * 4) as usize;
        let pixels = unsafe { std::slice::from_raw_parts_mut(self.bits, len) };
        let (width, height, radius) = (self.width as f32, self.height as f32, radius.max(0) as f32);
        for (index, pixel) in pixels.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let x = (index % self.width as usize) as f32 + 0.5;
            let y = (index / self.width as usize) as f32 + 0.5;
            let alpha = (coverage(x, y, width, height, radius) * 255.0).round() as u32;
            // 预乘：RGB 先乘上 alpha，分层窗按 AC_SRC_ALPHA 合成才是对的
            for channel in pixel.iter_mut().take(3) {
                *channel = (u32::from(*channel) * alpha / 255) as u8;
            }
            pixel[3] = alpha as u8;
        }
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

/// 把这张位图贴到分层窗口上，`pos` 是窗口左上角的屏幕坐标。
pub(crate) fn present(hwnd: HWND, canvas: &Canvas, pos: (i32, i32)) -> Result<()> {
    let dst = POINT { x: pos.0, y: pos.1 };
    let size = SIZE {
        cx: canvas.width,
        cy: canvas.height,
    };
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

/// 像素中心落在圆角矩形里的覆盖度（0 全外、1 全内）；只给圆角的四个弧线做边缘过渡。
fn coverage(x: f32, y: f32, width: f32, height: f32, radius: f32) -> f32 {
    // 采样点离四条边都至少一个像素才画得清楚；边线以外的像素直接透明。
    if x < 0.0 || y < 0.0 || x > width || y > height {
        return 0.0;
    }
    let right = width - radius;
    let bottom = height - radius;
    let cx = if x < radius {
        radius
    } else if x > right {
        right
    } else {
        x
    };
    let cy = if y < radius {
        radius
    } else if y > bottom {
        bottom
    } else {
        y
    };
    if cx == x && cy == y {
        return 1.0;
    }
    let distance = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
    (radius - distance + 0.5).clamp(0.0, 1.0)
}

fn fill_round_rect(
    hdc: HDC,
    (left, top, right, bottom): (i32, i32, i32, i32),
    color: COLORREF,
    radius: i32,
) {
    let diameter = (radius * 2).max(1);
    unsafe {
        let region = CreateRoundRectRgn(left, top, right, bottom, diameter, diameter);
        let brush = CreateSolidBrush(color);
        let _ = FillRgn(hdc, region, brush);
        let _ = DeleteObject(brush.into());
        let _ = DeleteObject(region.into());
    }
}

#[cfg(test)]
impl Canvas {
    /// 测试里把画好的像素拿出来存 PNG，人工看一眼。
    pub(crate) fn pixels(&self) -> &[u8] {
        let len = (self.width * self.height * 4) as usize;
        unsafe { std::slice::from_raw_parts(self.bits, len) }
    }

    pub(crate) fn size(&self) -> (i32, i32) {
        (self.width, self.height)
    }
}
