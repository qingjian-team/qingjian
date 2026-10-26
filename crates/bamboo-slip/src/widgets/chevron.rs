//! 下拉框右边朝下的箭头。

use tiny_skia::PathBuilder;

use crate::canvas::Canvas;
use crate::color::Color;
use crate::scene::Icon;

#[derive(Debug)]
pub(super) struct Chevron;

impl Icon for Chevron {
    fn draw(&self, canvas: &mut Canvas, x: f32, y: f32, size: f32, color: Color) {
        let mut path = PathBuilder::new();
        path.move_to(x + size * 0.22, y + size * 0.38);
        path.line_to(x + size * 0.5, y + size * 0.66);
        path.line_to(x + size * 0.78, y + size * 0.38);
        if let Some(path) = path.finish() {
            canvas.stroke_outline(&path, size * 0.13, color);
        }
    }
}
