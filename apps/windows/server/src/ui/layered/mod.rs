//! 分层窗口合成：圆角背景 + 四周柔和阴影 + 一段 GDI 内容，合成进一张预乘 alpha 的 BGRA 位图，
//! `UpdateLayeredWindow` 一次贴上。候选窗口与状态条共用。位图在 [`Canvas`]，内容圆角矩形在 [`RoundRect`]。

mod canvas;
mod round_rect;

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

    let dst = POINT {
        x: layered.win_pos.0,
        y: layered.win_pos.1,
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
