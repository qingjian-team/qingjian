//! 勾选框里的对勾。

use tiny_skia::PathBuilder;

use crate::canvas::Canvas;
use crate::color::Color;
use crate::scene::Icon;

#[derive(Debug)]
pub(super) struct CheckMark;

impl Icon for CheckMark {
    fn draw(&self, canvas: &mut Canvas, x: f32, y: f32, size: f32, color: Color) {
        let mut path = PathBuilder::new();
        path.move_to(x + size * 0.26, y + size * 0.52);
        path.line_to(x + size * 0.43, y + size * 0.69);
        path.line_to(x + size * 0.75, y + size * 0.33);
        if let Some(path) = path.finish() {
            canvas.stroke_outline(&path, size * 0.13, color);
        }
    }
}
