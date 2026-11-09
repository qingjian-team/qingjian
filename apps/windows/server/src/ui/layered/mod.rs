//! 分层窗口合成：圆角背景 + 四周柔和阴影 + 一段 GDI 内容，合成进一张预乘 alpha 的 BGRA 位图，
//! `UpdateLayeredWindow` 一次贴上。候选窗口与状态条共用。位图在 [`Canvas`]，内容圆角矩形在 [`RoundRect`]。
//! 青简渲染器画好的整张位图（已含阴影）走 [`present`]，只做 RGBA → BGRA 再贴。

mod canvas;
mod round_rect;

use qingjian_render::Pixmap;
use windows::Win32::Foundation::{COLORREF, E_INVALIDARG, HWND, POINT, RECT, SIZE};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION, HDC, SetViewportOrgEx,
};
use windows::Win32::UI::WindowsAndMessaging::{ULW_ALPHA, UpdateLayeredWindow};
use windows::core::{Error, Result};

use self::canvas::Canvas;
use self::round_rect::RoundRect;

/// 内容四周留给阴影的宽度（逻辑像素）。
const SHADOW_MARGIN: i32 = 16;

/// 定向主阴影（光从上方来）的最浓 alpha：底部拿满、两侧减半、顶部为 0。
const KEY_MAX: f64 = 65.0;

/// 环境光晕的最浓 alpha，四边等浓。
const AMBIENT_MAX: f64 = 18.0;

/// 阴影留白的物理像素宽度（`dpi` 96 为 100%）。
pub(super) fn shadow_margin(dpi: u32) -> i32 {
    ((SHADOW_MARGIN * dpi as i32) / 96).max(1)
}

/// 一次合成的输入。
pub(super) struct Layered<'a> {
    /// 内容尺寸（不含阴影留白）。
    pub content: (i32, i32),

    /// 阴影留白（= [`shadow_margin`]）。
    pub margin: i32,

    /// 窗口左上角屏幕坐标（= 内容左上角 − 留白）。
    pub win_pos: (i32, i32),

    /// 窗口尺寸（= 内容 + 2·留白）。
    pub win_size: (i32, i32),

    /// 内容背景色。
    pub background: COLORREF,

    /// 内容圆角半径。
    pub corner_radius: i32,

    /// 在内容坐标系里画内容；`client` 是 `{0, 0, w, h}`。
    pub paint: &'a dyn Fn(HDC, RECT),
}

/// 合成一帧并贴到分层窗口上。
pub(super) fn composite(hwnd: HWND, layered: &Layered) -> Result<()> {
    let (w, h) = layered.win_size;
    if w <= 0 || h <= 0 {
        return Err(Error::from(E_INVALIDARG));
    }
    let mut canvas = Canvas::new(w, h)?;
    let round = RoundRect::content(layered.content, layered.margin, layered.corner_radius);
    fill_shadow_and_background(
        canvas.pixels(),
        w,
        h,
        &round,
        layered.background,
        layered.margin,
    );

    // 视口原点挪到内容左上，内容闭包按自己的坐标画。
    let hdc = canvas.dc();
    unsafe {
        let _ = SetViewportOrgEx(hdc, layered.margin, layered.margin, None);
    }
    let client = RECT {
        left: 0,
        top: 0,
        right: layered.content.0,
        bottom: layered.content.1,
    };
    (layered.paint)(hdc, client);
    unsafe {
        let _ = SetViewportOrgEx(hdc, 0, 0, None);
    }
    // GDI 只写 RGB、把碰到的像素 alpha 留成 0（分层窗口里会全透明），画完把内容区补回 255。
    restore_content_alpha(canvas.pixels(), w, h, &round);
    update(hwnd, canvas.dc(), layered.win_pos, (w, h), 255)
}

/// 把渲染器出的预乘 RGBA 位图（已含阴影边）贴到分层窗口上，`win_pos` 是位图左上角的屏幕坐标。
pub(super) fn present(hwnd: HWND, pixmap: &Pixmap, win_pos: (i32, i32)) -> Result<()> {
    present_with_alpha(hwnd, pixmap, win_pos, 255)
}

/// 同 [`present`]，整个窗口再乘一个不透明度（255 为不透明）。
pub(super) fn present_with_alpha(
    hwnd: HWND,
    pixmap: &Pixmap,
    win_pos: (i32, i32),
    alpha: u8,
) -> Result<()> {
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
    update(hwnd, canvas.dc(), win_pos, (w, h), alpha)
}

/// `UpdateLayeredWindow`：整张位图按预乘 alpha 贴上并挪到 `win_pos`，`alpha` 是整窗不透明度。
fn update(
    hwnd: HWND,
    hdc: HDC,
    win_pos: (i32, i32),
    win_size: (i32, i32),
    alpha: u8,
) -> Result<()> {
    let (w, h) = win_size;
    let dst = POINT {
        x: win_pos.0,
        y: win_pos.1,
    };
    let size = SIZE { cx: w, cy: h };
    let src = POINT { x: 0, y: 0 };
    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: alpha,
        AlphaFormat: AC_SRC_ALPHA as u8,
    };
    unsafe {
        UpdateLayeredWindow(
            hwnd,
            None,
            Some(&dst),
            Some(&size),
            Some(hdc),
            Some(&src),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        )
    }
}

/// 圆角矩形内填背景色（alpha 255），外面画黑色阴影。阴影纯黑、背景不透明，所以不用真做预乘。
/// 阴影从内容边缘就开始淡出，不偏移、不填实心色带（否则底边多一道生硬暗带）。
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
