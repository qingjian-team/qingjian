//! 候选窗口的分层窗口表面：把一帧内容合成进一张预乘 alpha 的 BGRA 位图，`UpdateLayeredWindow` 贴上去。
//!
//! 为什么不用 `CS_DROPSHADOW`：那套只在右 / 下投一道硬阴影，跟 macOS「四边柔和、候选框浮在应用之上」的观感
//! 差得远。分层窗口能逐像素控 alpha，于是四周自己画一圈由内向外淡出的柔和阴影，内容区不透明。
//!
//! 合成分三步（都在一张位图上）：
//! 1. **像素循环铺底**：内容圆角矩形内填背景色（alpha 255）；矩形外画黑色柔和阴影。阴影是纯黑，预乘后
//!    仍是 `(0,0,0,a)`，背景 alpha 255 时 RGB 不用缩放——所以全程不需要真做预乘乘法。**阴影按光从上方来
//!    的方向感画**（对齐 macOS）：阴影一律从内容边缘就开始柔和淡出
//!    （不偏移、不填实心色带，否则底边会多一道生硬暗带），叠四周等浓但很淡的环境光晕（`AMBIENT_MAX`）与按
//!    「朝下程度」加权的定向主阴影（`KEY_MAX`：底满、两侧减半、顶为 0）。于是底边最实、两侧次之、顶边几乎只剩
//!    环境光晕——顶边不发灰、不挡应用里显示的 preedit 拼音，同时四边都「浮」着。
//! 2. **GDI 画内容**：`SetViewportOrgEx` 把原点挪到内容左上（`margin, margin`），[`view::paint`] 的坐标不用改。
//! 3. **补 alpha**：GDI 的 `TextOutW` / `FillRect` / `FillRgn` 只写 RGB、把碰到的像素 alpha 留成 0，
//!    在分层窗口里会全透明（文字 / 高亮消失）。画完把内容区所有像素 alpha 补成 255（内容不透明；预乘下
//!    alpha=255 时 RGB 无需缩放），阴影区不动。

use std::ffi::c_void;

use windows::Win32::Foundation::{COLORREF, HWND, POINT, RECT, SIZE};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, SelectObject,
    SetViewportOrgEx,
};
use windows::Win32::UI::WindowsAndMessaging::{ULW_ALPHA, UpdateLayeredWindow};
use windows::core::{Error, Result};

use super::RenderData;
use super::view;

/// 定向主阴影的最浓 alpha（0–255）：光从上方来，方向权重让底部拿满、两侧减半、顶部为 0（只剩环境光晕）。
const KEY_MAX: f64 = 65.0;

/// 四周环境光晕的最浓 alpha（0–255）：很淡、各边等浓，让候选框四边都「浮」着；顶边只剩它。
const AMBIENT_MAX: f64 = 18.0;

/// 把一帧内容合成成分层位图并贴到窗口上：定位到 `win_pos`、尺寸 `win_size`（= 内容 + 2·margin）。
/// 内容区在位图里的左上角是 `(margin, margin)`；四周留 `margin` 给阴影。
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
        return Err(Error::from(windows::Win32::Foundation::E_INVALIDARG));
    }

    // top-down（biHeight 取负）32bpp DIB，像素即 BGRA，逐字节可写。
    let header = BITMAPINFOHEADER {
        biSize: core::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: w,
        biHeight: -h,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        ..Default::default()
    };
    let bmi = BITMAPINFO {
        bmiHeader: header,
        ..Default::default()
    };

    let mut bits: *mut c_void = core::ptr::null_mut();
    // SAFETY: bmi 已填好；CreateDIBSection 把像素缓冲地址写进 bits。
    let dib = unsafe { CreateDIBSection(None, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)? };
    if bits.is_null() {
        // SAFETY: dib 由上面成功创建。
        unsafe {
            let _ = DeleteObject(dib.into());
        }
        return Err(Error::from(windows::Win32::Foundation::E_FAIL));
    }

    // SAFETY: DIB 有 w*h 个像素、每像素 4 字节，缓冲连续。
    let pixels = unsafe { std::slice::from_raw_parts_mut(bits.cast::<u8>(), (w * h * 4) as usize) };

    let round = RoundRect::content(content, margin, data.theme.corner_radius);
    fill_shadow_and_background(pixels, w, h, &round, data.theme.background, margin);

    // SAFETY: memdc / dib 都由本函数管理，用完即删。
    unsafe {
        let memdc = CreateCompatibleDC(None);
        let old = SelectObject(memdc, dib.into());

        // 原点挪到内容左上，view 的坐标（以内容 0,0 为基准）无需改动。
        let _ = SetViewportOrgEx(memdc, margin, margin, None);
        let client = RECT {
            left: 0,
            top: 0,
            right: content.0,
            bottom: content.1,
        };
        view::paint(memdc, data, client);
        let _ = SetViewportOrgEx(memdc, 0, 0, None);

        // GDI 把画过的像素 alpha 留成 0，补回内容区不透明。
        restore_content_alpha(pixels, w, h, &round);

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
        let updated = UpdateLayeredWindow(
            hwnd,
            None,
            Some(&dst),
            Some(&size),
            Some(memdc),
            Some(&src),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        );

        SelectObject(memdc, old);
        let _ = DeleteDC(memdc);
        let _ = DeleteObject(dib.into());
        updated
    }
}

/// 内容圆角矩形（位图坐标，含各边 `margin` 偏移），用来判定像素在内容内 / 外并算阴影距离。
struct RoundRect {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
    radius: f64,
}

impl RoundRect {
    /// 由内容尺寸、阴影边距与圆角半径构造：内容左上角落在 `(margin, margin)`。
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

    /// 点到圆角矩形的有符号距离：内部为负、边界为 0、外部为正的欧氏距离。像素取中心 `(x+0.5, y+0.5)`。
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

    /// 外部一点相对矩形的「朝下程度」：从最近边指向该点的方向的竖直分量映射到 `0..1`——正上方 0、正左 /
    /// 正右 0.5、正下 1。定向主阴影按它调浓度（光从上方来，越靠下越实）。
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

/// 逐像素铺底：内容圆角矩形内填背景色（不透明），矩形外画「光从上方」的方向阴影——四周等浓的淡环境光晕
/// 叠上按「朝下程度」加权的定向主阴影，底实顶虚、处处柔和。
fn fill_shadow_and_background(
    pixels: &mut [u8],
    w: i32,
    h: i32,
    round: &RoundRect,
    background: COLORREF,
    margin: i32,
) {
    // COLORREF 低位到高位是 R、G、B；DIB 每像素字节序是 B、G、R、A。
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
                // 一律从内容边缘就开始柔和淡出（无偏移、无实心色带）：环境光晕四周等浓，定向主阴影按朝下
                // 程度加权——底边拿满、两侧减半、顶边为 0，于是底实顶虚且处处柔和。
                let fall = (1.0 - d / margin).max(0.0);
                let fall = fall * fall;
                let ambient = AMBIENT_MAX * fall;
                let key = KEY_MAX * round.down_weight(x, y) * fall;
                let alpha = (ambient + key).min(255.0) as u8;
                pixels[idx] = 0;
                pixels[idx + 1] = 0;
                pixels[idx + 2] = 0;
                pixels[idx + 3] = alpha;
            }
        }
    }
}

/// 把内容区（圆角矩形内）所有像素 alpha 补成 255：GDI 画字 / 填色只写 RGB、alpha 留 0，不补会全透明。
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
