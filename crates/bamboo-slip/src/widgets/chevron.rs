//! 下拉框与步进钮上的箭头：朝下、朝上一个，或上下一对（macOS 的弹出按钮）。

use tiny_skia::PathBuilder;

use crate::canvas::Canvas;
use crate::color::Color;
use crate::scene::Icon;

#[derive(Debug, Clone, Copy)]
pub(super) enum Chevron {
    Down,

    Up,

    UpDown,
}

impl Icon for Chevron {
    fn draw(&self, canvas: &mut Canvas, x: f32, y: f32, size: f32, color: Color) {
        let mut path = PathBuilder::new();
        match self {
            Chevron::Down => {
                path.move_to(x + size * 0.22, y + size * 0.38);
                path.line_to(x + size * 0.5, y + size * 0.66);
                path.line_to(x + size * 0.78, y + size * 0.38);
            }
            Chevron::Up => {
                path.move_to(x + size * 0.22, y + size * 0.62);
                path.line_to(x + size * 0.5, y + size * 0.34);
                path.line_to(x + size * 0.78, y + size * 0.62);
            }
            Chevron::UpDown => {
                path.move_to(x + size * 0.28, y + size * 0.36);
                path.line_to(x + size * 0.5, y + size * 0.14);
                path.line_to(x + size * 0.72, y + size * 0.36);
                path.move_to(x + size * 0.28, y + size * 0.64);
                path.line_to(x + size * 0.5, y + size * 0.86);
                path.line_to(x + size * 0.72, y + size * 0.64);
            }
        }
        if let Some(path) = path.finish() {
            canvas.stroke_outline(&path, size * 0.13, color);
        }
    }
}
