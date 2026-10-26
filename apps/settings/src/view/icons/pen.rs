//! 16 单位网格上的画笔：坐标换到画布像素，画好的路径分描边与填充两堆。

use bamboo_slip::tiny_skia::{Path, PathBuilder};

/// 16 单位网格上的画笔：坐标换到画布像素。
pub(super) struct Pen {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) unit: f32,
    pub(super) strokes: Vec<Path>,
    pub(super) fills: Vec<Path>,
}

impl Pen {
    pub(super) fn p(&self, x: f32, y: f32) -> (f32, f32) {
        (self.x + x * self.unit, self.y + y * self.unit)
    }

    pub(super) fn line(&mut self, points: &[(f32, f32)]) {
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

    pub(super) fn circle(&mut self, cx: f32, cy: f32, r: f32, fill: bool) {
        let (x, y) = self.p(cx, cy);
        if let Some(path) = PathBuilder::from_circle(x, y, r * self.unit) {
            if fill {
                self.fills.push(path);
            } else {
                self.strokes.push(path);
            }
        }
    }

    pub(super) fn round_rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, fill: bool) {
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

    pub(super) fn curve(&mut self, start: (f32, f32), segments: &[[(f32, f32); 3]]) {
        if let Some(path) = self.curve_path(start, segments, false) {
            self.strokes.push(path);
        }
    }

    /// 曲线连回起点围成的面。
    pub(super) fn fill_curve(&mut self, start: (f32, f32), segments: &[[(f32, f32); 3]]) {
        if let Some(path) = self.curve_path(start, segments, true) {
            self.fills.push(path);
        }
    }

    fn curve_path(
        &self,
        start: (f32, f32),
        segments: &[[(f32, f32); 3]],
        close: bool,
    ) -> Option<Path> {
        let mut path = PathBuilder::new();
        let (x, y) = self.p(start.0, start.1);
        path.move_to(x, y);
        for [a, b, c] in segments {
            let (a, b, c) = (self.p(a.0, a.1), self.p(b.0, b.1), self.p(c.0, c.1));
            path.cubic_to(a.0, a.1, b.0, b.1, c.0, c.1);
        }
        if close {
            path.close();
        }
        path.finish()
    }
}
