//! 分层窗口表面：把一帧内容合成进一张预乘 alpha 的 BGRA 位图，`UpdateLayeredWindow` 贴上去。
//!
//! 三步：① 逐像素铺底——内容圆角矩形内填背景色（alpha 255），矩形外画黑色柔和阴影；阴影是纯黑、背景不透明，
//! 所以不需要真做预乘乘法。② GDI 画内容——`SetViewportOrgEx` 把原点挪到内容左上，[`view::paint`] 的坐标不用改。
//! ③ 补 alpha——GDI 只写 RGB、把碰到的像素 alpha 留成 0，在分层窗口里会全透明，画完把内容区 alpha 补回 255。
//!
//! 阴影按「光从上方来」画（对齐 macOS）：四周等浓的淡环境光晕，叠上按「朝下程度」加权的定向主阴影，
//! 底实顶虚。一律从内容边缘就开始淡出，不偏移、不填实心色带，否则底边会多一道生硬暗带。

use windows::Win32::Foundation::{COLORREF, E_FAIL, E_INVALIDARG, HWND, POINT, RECT, SIZE};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, GdiFlush,
    HBITMAP, HDC, HGDIOBJ, SelectObject, SetViewportOrgEx,
};
use windows::Win32::UI::WindowsAndMessaging::{ULW_ALPHA, UpdateLayeredWindow};
use windows::core::{Error, Result};

use super::RenderData;
use super::view;

/// 定向主阴影的最浓 alpha：底部拿满、两侧减半、顶部为 0。
const KEY_MAX: f64 = 65.0;

/// 环境光晕的最浓 alpha：很淡、四边等浓。
const AMBIENT_MAX: f64 = 18.0;

/// 合成一帧并贴到窗口上：窗口在 `win_pos`、尺寸 `win_size`（= 内容 + 2·margin），内容左上角在位图的 `(margin, margin)`。
pub(super) fn update(
    hwnd: HWND,
    data: &RenderData,
    content: (i32, i32),
    margin: i32,
    win_pos: (i32, i32),
    win_size: (i32, i32),
) -> Result<()> {
    let (w, h) = win_size;
    if w <= 0 || h <= 0 {
        return Err(Error::from(E_INVALIDARG));
    }
    let mut canvas = Canvas::new(w, h)?;
    let round = RoundRect::content(content, margin, data.theme.corner_radius);
    fill_shadow_and_background(canvas.pixels(), w, h, &round, data.theme.background, margin);

    let hdc = canvas.dc();
    // SAFETY: hdc 有效，只改视口原点。
    unsafe {
        let _ = SetViewportOrgEx(hdc, margin, margin, None);
    }
    let client = RECT {
        left: 0,
        top: 0,
        right: content.0,
        bottom: content.1,
    };
    view::paint(hdc, data, client);
    // SAFETY: 同上。
    unsafe {
        let _ = SetViewportOrgEx(hdc, 0, 0, None);
    }
    restore_content_alpha(canvas.pixels(), w, h, &round);

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
    // SAFETY: hwnd 是分层窗口；memdc 里选着位图；各结构体有效。
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

/// 一张 top-down 32bpp DIB 与选进了它的内存 DC，`Drop` 里还原并删除。
/// 像素只能经 [`Self::pixels`] 访问（`&mut self`），保证 GDI 在画的时候没有 Rust 侧的切片指着同一块内存。
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
        // SAFETY: bmi 已填好；像素缓冲地址写进 bits。
        let dib = unsafe { CreateDIBSection(None, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)? };
        // SAFETY: dib 刚创建；memdc 由本对象管理。
        let (memdc, previous) = unsafe {
            let memdc = CreateCompatibleDC(None);
            (memdc, SelectObject(memdc, dib.into()))
        };
        if bits.is_null() || memdc.is_invalid() {
            // SAFETY: 释放刚建的资源。
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

    /// 像素（BGRA，top-down）。先 `GdiFlush` 把批量的 GDI 调用落到位图上。
    fn pixels(&mut self) -> &mut [u8] {
        // SAFETY: 缓冲由 CreateDIBSection 分配，w*h*4 字节连续，随 dib 存活；`&mut self` 保证独占。
        unsafe {
            let _ = GdiFlush();
            std::slice::from_raw_parts_mut(self.bits, self.len)
        }
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        // SAFETY: 都由本对象创建。
        unsafe {
            SelectObject(self.memdc, self.previous);
            let _ = DeleteDC(self.memdc);
            let _ = DeleteObject(self.dib.into());
        }
    }
}

/// 内容圆角矩形（位图坐标）。
struct RoundRect {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
    radius: f64,
}

impl RoundRect {
    fn content(content: (i32, i32), margin: i32, radius: i32) -> Self {
        let left = margin as f64;
        let top = margin as f64;
        Self {
            left,
            top,
            right: left + content.0 as f64,
            bottom: top + content.1 as f64,
            radius: radius as f64,
        }
    }

    /// 像素中心到圆角矩形的有符号距离：内部为负、边界为 0、外部为正。
    fn distance(&self, x: i32, y: i32) -> f64 {
        let px = x as f64 + 0.5;
        let py = y as f64 + 0.5;
        let cx = (self.left + self.right) / 2.0;
        let cy = (self.top + self.bottom) / 2.0;
        let half_w = (self.right - self.left) / 2.0;
        let half_h = (self.bottom - self.top) / 2.0;
        let dx = (px - cx).abs() - half_w + self.radius;
        let dy = (py - cy).abs() - half_h + self.radius;
        let outside = dx.max(0.0).hypot(dy.max(0.0));
        let inside = dx.max(dy).min(0.0);
        inside + outside - self.radius
    }

    /// 外部一点的「朝下程度」：从最近边指向该点的方向的竖直分量映射到 `0..1`，正上方 0、两侧 0.5、正下 1。
    fn down_weight(&self, x: i32, y: i32) -> f64 {
        let px = x as f64 + 0.5;
        let py = y as f64 + 0.5;
        let vx = px - px.clamp(self.left, self.right);
        let vy = py - py.clamp(self.top, self.bottom);
        let len = vx.hypot(vy);
        if len <= 0.0 {
            0.5
        } else {
            (vy / len + 1.0) * 0.5
        }
    }
}

fn fill_shadow_and_background(
    pixels: &mut [u8],
    w: i32,
    h: i32,
    round: &RoundRect,
    background: COLORREF,
    margin: i32,
) {
    // COLORREF 低位到高位是 R、G、B；DIB 每像素是 B、G、R、A。
    let bg = background.0;
    let bg_r = (bg & 0xFF) as u8;
    let bg_g = ((bg >> 8) & 0xFF) as u8;
    let bg_b = ((bg >> 16) & 0xFF) as u8;
    let margin = (margin as f64).max(1.0);
    for y in 0..h {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            let d = round.distance(x, y);
            if d <= 0.0 {
                pixels[idx] = bg_b;
                pixels[idx + 1] = bg_g;
                pixels[idx + 2] = bg_r;
                pixels[idx + 3] = 255;
            } else {
                let fall = (1.0 - d / margin).max(0.0);
                let fall = fall * fall;
                let ambient = AMBIENT_MAX * fall;
                let key = KEY_MAX * round.down_weight(x, y) * fall;
                pixels[idx] = 0;
                pixels[idx + 1] = 0;
                pixels[idx + 2] = 0;
                pixels[idx + 3] = (ambient + key).min(255.0) as u8;
            }
        }
    }
}

/// 把内容区所有像素 alpha 补成 255。
fn restore_content_alpha(pixels: &mut [u8], w: i32, h: i32, round: &RoundRect) {
    for y in 0..h {
        for x in 0..w {
            if round.distance(x, y) <= 0.0 {
                let idx = ((y * w + x) * 4) as usize;
                pixels[idx + 3] = 255;
            }
        }
    }
}
