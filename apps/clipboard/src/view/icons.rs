//! 面板的图标：按钮用系统图标字体，搜索框要 [`Icon`] 就自己画放大镜。

use std::f32::consts::FRAC_1_SQRT_2;
use std::sync::Arc;

use bamboo_slip::tiny_skia::{Path, PathBuilder};
use bamboo_slip::{Canvas, Color, Icon};

/// Segoe Fluent Icons 与 Segoe MDL2 Assets 共用的码位。
pub(super) const PASTE: &str = "\u{E77F}";

pub(super) const COPY: &str = "\u{E8C8}";

/// 右键菜单的图标，16 单位见方的网格上画。
#[derive(Debug, Clone, Copy)]
enum MenuGlyph {
    Paste,
    Copy,
    Edit,
    Delete,
}

/// 与 `menu_items` 同序。
pub(super) fn menu_icons() -> [Arc<dyn Icon>; 4] {
    [
        Arc::new(MenuGlyph::Paste),
        Arc::new(MenuGlyph::Copy),
        Arc::new(MenuGlyph::Edit),
        Arc::new(MenuGlyph::Delete),
    ]
}

fn polyline(path: &mut PathBuilder, to: impl Fn(f32, f32) -> (f32, f32), points: &[(f32, f32)]) {
    for (index, (x, y)) in points.iter().enumerate() {
        let (x, y) = to(*x, *y);
        if index == 0 {
            path.move_to(x, y);
        } else {
            path.line_to(x, y);
        }
    }
}

impl MenuGlyph {
    fn path(self, x: f32, y: f32, size: f32) -> Option<Path> {
        let to = |px: f32, py: f32| (x + px * size / 16.0, y + py * size / 16.0);
        let mut path = PathBuilder::new();
        let lines: &[&[(f32, f32)]] = match self {
            Self::Paste => &[
                &[
                    (5.5, 3.0),
                    (3.0, 3.0),
                    (3.0, 14.5),
                    (13.0, 14.5),
                    (13.0, 3.0),
                    (10.5, 3.0),
                ],
                &[(5.5, 1.5), (10.5, 1.5), (10.5, 4.5), (5.5, 4.5), (5.5, 1.5)],
            ],
            Self::Copy => &[
                &[
                    (5.5, 1.5),
                    (14.0, 1.5),
                    (14.0, 11.5),
                    (5.5, 11.5),
                    (5.5, 1.5),
                ],
                &[
                    (5.5, 4.5),
                    (2.0, 4.5),
                    (2.0, 14.5),
                    (10.5, 14.5),
                    (10.5, 11.5),
                ],
            ],
            Self::Edit => &[
                &[
                    (2.0, 14.0),
                    (2.5, 10.5),
                    (11.0, 2.0),
                    (14.0, 5.0),
                    (5.5, 13.5),
                    (2.0, 14.0),
                ],
                &[(9.5, 3.5), (12.5, 6.5)],
            ],
            Self::Delete => &[
                &[(1.5, 4.0), (14.5, 4.0)],
                &[(6.0, 4.0), (6.0, 1.5), (10.0, 1.5), (10.0, 4.0)],
                &[(3.0, 4.0), (4.0, 14.5), (12.0, 14.5), (13.0, 4.0)],
                &[(6.5, 7.0), (6.5, 11.5)],
                &[(9.5, 7.0), (9.5, 11.5)],
            ],
        };
        for line in lines {
            polyline(&mut path, to, line);
        }
        path.finish()
    }
}

impl Icon for MenuGlyph {
    fn draw(&self, canvas: &mut Canvas, x: f32, y: f32, size: f32, color: Color) {
        if let Some(path) = self.path(x, y, size) {
            canvas.stroke_outline(&path, size * 0.08, color);
        }
    }
}

#[derive(Debug)]
pub(super) struct SearchGlyph;

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
