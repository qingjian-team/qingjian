//! [`MenuGlyph`]：右键菜单的图标。

use bamboo_slip::tiny_skia::{Path, PathBuilder};
use bamboo_slip::{Canvas, Color, Icon};

/// 右键菜单的图标，16 单位见方的网格上画。
#[derive(Debug, Clone, Copy)]
pub(super) enum MenuGlyph {
    Paste,
    Copy,
    Edit,
    Delete,
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
