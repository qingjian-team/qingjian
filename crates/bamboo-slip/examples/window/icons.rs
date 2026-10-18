//! 侧栏的线性图标：16 单位见方的网格上画，1.5 单位线宽、圆头圆角，按 `size` 缩放。

use bamboo_slip::tiny_skia::{Path, PathBuilder};
use bamboo_slip::{Canvas, Color, Icon};

#[derive(Debug, Clone, Copy)]
pub enum Glyph {
    General,
    Candidates,
    Shortcuts,
    Phrases,
    Fuzzy,
    Dictionary,
    AuxCode,
    Cloud,
    Stats,
    Advanced,
    About,
    Search,
}

pub const PAGE_GLYPHS: [Glyph; 11] = [
    Glyph::General,
    Glyph::Candidates,
    Glyph::Shortcuts,
    Glyph::Phrases,
    Glyph::Fuzzy,
    Glyph::Dictionary,
    Glyph::AuxCode,
    Glyph::Cloud,
    Glyph::Stats,
    Glyph::Advanced,
    Glyph::About,
];

/// 16 单位网格上的画笔：坐标换到画布像素。
struct Pen {
    x: f32,
    y: f32,
    unit: f32,
    strokes: Vec<Path>,
    fills: Vec<Path>,
}

impl Pen {
    fn p(&self, x: f32, y: f32) -> (f32, f32) {
        (self.x + x * self.unit, self.y + y * self.unit)
    }

    fn line(&mut self, points: &[(f32, f32)]) {
        let mut path = PathBuilder::new();
        for (index, &(x, y)) in points.iter().enumerate() {
            let (x, y) = self.p(x, y);
            if index == 0 {
                path.move_to(x, y);
            } else {
                path.line_to(x, y);
            }
        }
        if let Some(path) = path.finish() {
            self.strokes.push(path);
        }
    }

    fn circle(&mut self, cx: f32, cy: f32, r: f32, fill: bool) {
        let (x, y) = self.p(cx, cy);
        if let Some(path) = PathBuilder::from_circle(x, y, r * self.unit) {
            if fill {
                self.fills.push(path);
            } else {
                self.strokes.push(path);
            }
        }
    }

    fn round_rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, fill: bool) {
        let (x, y) = self.p(x, y);
        let (w, h, r) = (w * self.unit, h * self.unit, r * self.unit);
        let mut path = PathBuilder::new();
        path.move_to(x + r, y);
        path.line_to(x + w - r, y);
        path.quad_to(x + w, y, x + w, y + r);
        path.line_to(x + w, y + h - r);
        path.quad_to(x + w, y + h, x + w - r, y + h);
        path.line_to(x + r, y + h);
        path.quad_to(x, y + h, x, y + h - r);
        path.line_to(x, y + r);
        path.quad_to(x, y, x + r, y);
        path.close();
        if let Some(path) = path.finish() {
            if fill {
                self.fills.push(path);
            } else {
                self.strokes.push(path);
            }
        }
    }

    fn curve(&mut self, start: (f32, f32), segments: &[[(f32, f32); 3]]) {
        let mut path = PathBuilder::new();
        let (x, y) = self.p(start.0, start.1);
        path.move_to(x, y);
        for [a, b, c] in segments {
            let (a, b, c) = (self.p(a.0, a.1), self.p(b.0, b.1), self.p(c.0, c.1));
            path.cubic_to(a.0, a.1, b.0, b.1, c.0, c.1);
        }
        if let Some(path) = path.finish() {
            self.strokes.push(path);
        }
    }
}

impl Icon for Glyph {
    fn draw(&self, canvas: &mut Canvas, x: f32, y: f32, size: f32, color: Color) {
        let mut pen = Pen {
            x,
            y,
            unit: size / 16.0,
            strokes: Vec::new(),
            fills: Vec::new(),
        };
        match self {
            // 三根滑杆
            Glyph::General => {
                for (row, knob) in [(4.0, 10.5), (8.0, 5.0), (12.0, 9.0)] {
                    pen.line(&[(2.0, row), (14.0, row)]);
                    pen.circle(knob, row, 1.6, true);
                }
            }
            // 候选窗：窗框里一条高亮行
            Glyph::Candidates => {
                pen.round_rect(1.75, 2.75, 12.5, 10.5, 2.0, false);
                pen.round_rect(4.0, 6.0, 5.5, 2.5, 1.0, true);
                pen.line(&[(4.0, 10.75), (11.5, 10.75)]);
            }
            // 键盘
            Glyph::Shortcuts => {
                pen.round_rect(1.25, 3.75, 13.5, 8.5, 1.75, false);
                for column in [4.0, 6.67, 9.33, 12.0] {
                    pen.circle(column, 6.5, 0.7, true);
                }
                pen.line(&[(5.0, 9.5), (11.0, 9.5)]);
            }
            // 对话框里两行字
            Glyph::Phrases => {
                pen.line(&[
                    (4.0, 13.5),
                    (4.0, 11.5),
                    (3.0, 11.5),
                    (1.75, 10.25),
                    (1.75, 3.75),
                    (3.0, 2.5),
                    (13.0, 2.5),
                    (14.25, 3.75),
                    (14.25, 10.25),
                    (13.0, 11.5),
                    (7.0, 11.5),
                    (4.0, 13.5),
                ]);
                pen.line(&[(4.75, 5.75), (11.25, 5.75)]);
                pen.line(&[(4.75, 8.25), (9.0, 8.25)]);
            }
            // 两道起伏的声波
            Glyph::Fuzzy => {
                for row in [5.5, 10.5] {
                    pen.curve(
                        (1.5, row),
                        &[
                            [(3.5, row - 3.0), (5.5, row - 3.0), (7.5, row)],
                            [(9.5, row + 3.0), (11.5, row + 3.0), (14.5, row)],
                        ],
                    );
                }
            }
            // 翻开的书
            Glyph::Dictionary => {
                pen.line(&[(8.0, 4.0), (8.0, 13.5)]);
                pen.line(&[
                    (8.0, 4.0),
                    (5.5, 2.75),
                    (1.75, 2.75),
                    (1.75, 12.25),
                    (5.5, 12.25),
                    (8.0, 13.5),
                ]);
                pen.line(&[
                    (8.0, 4.0),
                    (10.5, 2.75),
                    (14.25, 2.75),
                    (14.25, 12.25),
                    (10.5, 12.25),
                    (8.0, 13.5),
                ]);
            }
            // 井号
            Glyph::AuxCode => {
                pen.line(&[(6.0, 2.0), (5.0, 14.0)]);
                pen.line(&[(11.0, 2.0), (10.0, 14.0)]);
                pen.line(&[(2.5, 5.75), (13.75, 5.75)]);
                pen.line(&[(2.25, 10.25), (13.5, 10.25)]);
            }
            // 云
            Glyph::Cloud => {
                pen.curve(
                    (4.5, 12.75),
                    &[
                        [(2.25, 12.75), (1.0, 11.25), (1.25, 9.5)],
                        [(1.5, 7.75), (3.0, 6.75), (4.5, 7.0)],
                        [(5.0, 4.0), (7.5, 2.75), (10.0, 3.5)],
                        [(12.25, 4.25), (13.25, 6.25), (12.75, 8.0)],
                        [(14.75, 8.5), (15.25, 10.5), (14.25, 11.75)],
                        [(13.5, 12.5), (12.75, 12.75), (11.75, 12.75)],
                    ],
                );
                pen.line(&[(11.75, 12.75), (4.5, 12.75)]);
            }
            // 三根柱
            Glyph::Stats => {
                pen.line(&[(3.5, 13.0), (3.5, 9.0)]);
                pen.line(&[(8.0, 13.0), (8.0, 3.5)]);
                pen.line(&[(12.5, 13.0), (12.5, 6.5)]);
            }
            // 终端提示符
            Glyph::Advanced => {
                pen.round_rect(1.25, 2.25, 13.5, 11.5, 2.0, false);
                pen.line(&[(4.25, 6.0), (6.75, 8.0), (4.25, 10.0)]);
                pen.line(&[(8.5, 10.0), (11.75, 10.0)]);
            }
            // 圆圈里一个 i
            Glyph::About => {
                pen.circle(8.0, 8.0, 6.5, false);
                pen.circle(8.0, 5.0, 0.9, true);
                pen.line(&[(8.0, 7.5), (8.0, 11.25)]);
            }
            Glyph::Search => {
                pen.circle(7.0, 7.0, 4.75, false);
                pen.line(&[(10.5, 10.5), (14.0, 14.0)]);
            }
        }
        let width = 1.5 * pen.unit;
        for path in &pen.strokes {
            canvas.stroke_outline(path, width, color);
        }
        for path in &pen.fills {
            canvas.fill_path(path, color, bamboo_slip::tiny_skia::BlendMode::SourceOver);
        }
    }
}
