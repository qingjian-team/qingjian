//! 云联想的小云朵：两个圆拱加一条圆角底边的并集，只描边不填充，对应 macOS 的 SF Symbol `cloud`（18×13 挤进 13×13 方块）
//! 与 Windows 的 ☁ 字形。

use bamboo_slip::tiny_skia::{BlendMode, PathBuilder, Pixmap, Rect};

use bamboo_slip::{Canvas, Color, Icon};

/// 描边宽度相对边长的比例。
const STROKE_RATIO: f32 = 0.08;

#[derive(Debug)]
pub(super) struct Cloud;

impl Icon for Cloud {
    fn draw(&self, canvas: &mut Canvas, x: f32, y: f32, size: f32, color: Color) {
        draw_cloud(canvas, x, y, size, color);
    }
}

fn draw_cloud(canvas: &mut Canvas, x: f32, y: f32, size: f32, color: Color) {
    let side = size.ceil() as u32 + 1;
    let Some(icon) = Pixmap::new(side, side) else {
        return;
    };
    let stroke = (size * STROKE_RATIO).max(1.0);
    let mut layer = Canvas::from_pixmap(icon);
    if let Some(outer) = cloud_shape(size, 0.0) {
        layer.fill_path(&outer, color, BlendMode::SourceOver);
    }
    // 挖掉向内收一圈的同形，剩下的就是描边
    if let Some(inner) = cloud_shape(size, stroke) {
        layer.fill_path(&inner, Color::rgb(0, 0, 0), BlendMode::Clear);
    }
    let icon = layer.into_pixmap();
    canvas.blend_pixmap(x.round() as i32, y.round() as i32, &icon);
}

/// 云朵轮廓：向内收 `inset` 像素的版本用来挖空。坐标按边长归一化。
fn cloud_shape(size: f32, inset: f32) -> Option<bamboo_slip::tiny_skia::Path> {
    let s = size;
    let mut path = PathBuilder::new();
    let mut circle = |cx: f32, cy: f32, r: f32| {
        let radius = r * s - inset;
        if radius > 0.0 {
            path.push_circle(cx * s, cy * s, radius);
        }
    };
    // 左边小拱、右边大拱、底边两个圆角
    circle(0.34, 0.60, 0.23);
    circle(0.63, 0.50, 0.30);
    circle(0.25, 0.72, 0.16);
    circle(0.77, 0.72, 0.16);
    // 底边：把两个圆角之间填平
    let (top, bottom) = (0.60 * s, 0.88 * s - inset);
    if bottom > top
        && let Some(rect) = Rect::from_ltrb(0.25 * s, top, 0.77 * s, bottom)
    {
        path.push_rect(rect);
    }
    path.finish()
}
