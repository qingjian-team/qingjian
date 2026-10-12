//! 常用语候选右上角的小立方体：等轴测线框，只描边不填充（对齐微信键盘常用语标记）。

use tiny_skia::{PathBuilder, Pixmap};

use crate::canvas::Canvas;
use crate::color::Color;

/// 描边宽度相对边长的比例。
const STROKE_RATIO: f32 = 0.11;

/// 画在 `(x, y)` 为左上角、边长 `size` 像素的方块里。
pub(crate) fn draw_phrase(canvas: &mut Canvas, x: f32, y: f32, size: f32, color: Color) {
    let side = size.ceil() as u32 + 1;
    let Some(icon) = Pixmap::new(side, side) else {
        return;
    };
    let stroke = (size * STROKE_RATIO).max(1.0);
    let mut layer = Canvas::from_pixmap(icon);
    if let Some(path) = cube_path(size) {
        layer.stroke_path(&path, stroke, color);
    }
    let icon = layer.into_pixmap();
    canvas.blend_pixmap(x.round() as i32, y.round() as i32, &icon);
}

/// 等轴测线框立方体：前面正方形 + 顶面 / 右侧两条可见棱。
fn cube_path(size: f32) -> Option<tiny_skia::Path> {
    let s = size;
    let front = [
        (0.12 * s, 0.38 * s),
        (0.62 * s, 0.38 * s),
        (0.62 * s, 0.88 * s),
        (0.12 * s, 0.88 * s),
    ];
    let top_right = (0.88 * s, 0.12 * s);
    let top_left = (0.38 * s, 0.12 * s);
    let bottom_right = (0.88 * s, 0.62 * s);

    let mut path = PathBuilder::new();
    path.move_to(front[0].0, front[0].1);
    for p in &front[1..] {
        path.line_to(p.0, p.1);
    }
    path.close();

    path.move_to(front[0].0, front[0].1);
    path.line_to(top_left.0, top_left.1);
    path.line_to(top_right.0, top_right.1);
    path.line_to(front[1].0, front[1].1);

    path.move_to(front[1].0, front[1].1);
    path.line_to(top_right.0, top_right.1);
    path.line_to(bottom_right.0, bottom_right.1);
    path.line_to(front[2].0, front[2].1);

    path.finish()
}
