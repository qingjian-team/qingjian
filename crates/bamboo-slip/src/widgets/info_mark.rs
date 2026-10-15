//! 悬停说明的 ⓘ：一圈细线里一个 i。

use tiny_skia::PathBuilder;

use crate::canvas::Canvas;
use crate::color::Color;
use crate::scene::Icon;

#[derive(Debug)]
pub(super) struct InfoMark;

impl Icon for InfoMark {
    fn draw(&self, canvas: &mut Canvas, x: f32, y: f32, size: f32, color: Color) {
        let (cx, cy) = (x + size / 2.0, y + size / 2.0);
        let line = size * 0.09;
        if let Some(circle) = PathBuilder::from_circle(cx, cy, size / 2.0 - line / 2.0) {
            canvas.stroke_outline(&circle, line, color);
        }
        if let Some(dot) = PathBuilder::from_circle(cx, y + size * 0.3, size * 0.07) {
            canvas.fill_path(&dot, color, tiny_skia::BlendMode::SourceOver);
        }
        let mut stem = PathBuilder::new();
        stem.move_to(cx, y + size * 0.45);
        stem.line_to(cx, y + size * 0.74);
        if let Some(stem) = stem.finish() {
            canvas.stroke_outline(&stem, line * 1.2, color);
        }
    }
}
