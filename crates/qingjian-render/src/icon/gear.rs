//! 状态条的齿轮：八齿外圈加一个圆孔，只描边不填充，对应 Windows 端原先用的 ⚙ 字形（Segoe UI Symbol）。
//! 自己画是为了不受字体回退影响：Segoe UI Emoji 会把 U+2699 画成彩色。

use std::f32::consts::TAU;

use bamboo_slip::tiny_skia::{BlendMode, PathBuilder, Pixmap};

use bamboo_slip::{Canvas, Color, Icon};

/// 齿数。
const TEETH: usize = 8;

/// 描边宽度相对边长的比例。
const STROKE_RATIO: f32 = 0.09;

#[derive(Debug)]
pub(super) struct Gear;

impl Icon for Gear {
    fn draw(&self, canvas: &mut Canvas, x: f32, y: f32, size: f32, color: Color) {
        draw_gear(canvas, x, y, size, color);
    }
}

fn draw_gear(canvas: &mut Canvas, x: f32, y: f32, size: f32, color: Color) {
    let side = size.ceil() as u32 + 1;
    let Some(icon) = Pixmap::new(side, side) else {
        return;
    };
    let stroke = (size * STROKE_RATIO).max(1.0);
    let mut layer = Canvas::from_pixmap(icon);
    if let Some(outer) = gear_outline(size, 0.0) {
        layer.fill_path(&outer, color, BlendMode::SourceOver);
    }
    if let Some(inner) = gear_outline(size, stroke) {
        layer.fill_path(&inner, Color::rgb(0, 0, 0), BlendMode::Clear);
    }
    // 中间的孔：先挖空一圈再补一个内圈，就是描边的圆
    let center = size / 2.0;
    let hole = size * 0.18;
    let mut path = PathBuilder::new();
    path.push_circle(center, center, hole + stroke);
    if let Some(ring) = path.finish() {
        layer.fill_path(&ring, Color::rgb(0, 0, 0), BlendMode::Clear);
    }
    let mut path = PathBuilder::new();
    path.push_circle(center, center, hole + stroke);
    let mut inner = PathBuilder::new();
    inner.push_circle(center, center, hole);
    if let (Some(ring), Some(inner)) = (path.finish(), inner.finish()) {
        layer.fill_path(&ring, color, BlendMode::SourceOver);
        layer.fill_path(&inner, Color::rgb(0, 0, 0), BlendMode::Clear);
    }
    let icon = layer.into_pixmap();
    canvas.blend_pixmap(x.round() as i32, y.round() as i32, &icon);
}

/// 齿轮外轮廓：齿顶与齿根交替的折线，向内收 `inset` 像素的版本用来挖空。
fn gear_outline(size: f32, inset: f32) -> Option<bamboo_slip::tiny_skia::Path> {
    let center = size / 2.0;
    let outer = size * 0.48 - inset;
    let inner = size * 0.36 - inset;
    if inner <= 0.0 {
        return None;
    }
    // 每齿四个顶点：齿根起、齿顶起、齿顶止、齿根止；齿顶略窄于齿根
    let step = TAU / TEETH as f32;
    let root_half = step * 0.28;
    let tip_half = step * 0.18;
    let mut path = PathBuilder::new();
    for i in 0..TEETH {
        let mid = i as f32 * step;
        let points = [
            (mid - root_half, inner),
            (mid - tip_half, outer),
            (mid + tip_half, outer),
            (mid + root_half, inner),
        ];
        for (j, (angle, radius)) in points.iter().enumerate() {
            let px = center + radius * angle.cos();
            let py = center + radius * angle.sin();
            if i == 0 && j == 0 {
                path.move_to(px, py);
            } else {
                path.line_to(px, py);
            }
        }
    }
    path.close();
    path.finish()
}
