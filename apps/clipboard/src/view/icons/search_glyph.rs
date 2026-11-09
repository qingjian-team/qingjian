//! [`SearchGlyph`]：搜索框的放大镜。

use std::f32::consts::FRAC_1_SQRT_2;

use bamboo_slip::tiny_skia::PathBuilder;
use bamboo_slip::{Canvas, Color, Icon};

#[derive(Debug)]
pub(in crate::view) struct SearchGlyph;

impl Icon for SearchGlyph {
    fn draw(&self, canvas: &mut Canvas, x: f32, y: f32, size: f32, color: Color) {
        let radius = size * 0.34;
        let (cx, cy) = (x + size * 0.42, y + size * 0.42);
        let edge = radius * FRAC_1_SQRT_2;
        let mut path = PathBuilder::new();
        path.push_circle(cx, cy, radius);
        path.move_to(cx + edge, cy + edge);
        path.line_to(x + size * 0.88, y + size * 0.88);
        if let Some(path) = path.finish() {
            canvas.stroke_outline(&path, size * 0.08, color);
        }
    }
}
