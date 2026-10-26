//! 一个字在思源黑体里的轮廓：按笔画（闭合轮廓）分开存，可以只取其中几笔导出成 SVG 路径。

use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::{FontRef, MetadataProvider};

/// 汉字字面顶边在字体坐标里的高度（思源黑体 1000 单位一个字号，字面是基线上 880、下 120）。
/// 导出时以它为原点把 y 轴翻成向下，字面落在 0–1000 的方块里。
const FACE_TOP: f32 = 880.0;

#[derive(Debug, Clone, Copy)]
pub enum Segment {
    Move(f32, f32),

    Line(f32, f32),

    Quad(f32, f32, f32, f32),

    Cubic(f32, f32, f32, f32, f32, f32),
}

/// 一个字的全部轮廓，坐标已翻成 y 向下、字面方块 0–1000。
pub struct Outline {
    pub contours: Vec<Vec<Segment>>,
}

impl Outline {
    /// 字体里没有这个字返回 `None`。
    pub fn load(font: &FontRef, ch: char) -> Option<Self> {
        let glyph = font.charmap().map(ch)?;
        let outline = font.outline_glyphs().get(glyph)?;
        let mut pen = Pen::default();
        outline
            .draw(
                DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
                &mut pen,
            )
            .ok()?;
        Some(Self {
            contours: pen.contours,
        })
    }

    /// 选中的几笔（编号从 0 起）连成一条 SVG 路径；`None` 为全部。坐标取整。
    pub fn svg(&self, selected: Option<&[usize]>) -> String {
        let mut out = String::new();
        for (i, contour) in self.contours.iter().enumerate() {
            if selected.is_some_and(|keep| !keep.contains(&i)) {
                continue;
            }
            for segment in contour {
                let part = match *segment {
                    Segment::Move(x, y) => format!("M{} {}", r(x), r(y)),
                    Segment::Line(x, y) => format!("L{} {}", r(x), r(y)),
                    Segment::Quad(a, b, x, y) => format!("Q{} {} {} {}", r(a), r(b), r(x), r(y)),
                    Segment::Cubic(a, b, c, d, x, y) => {
                        format!("C{} {} {} {} {} {}", r(a), r(b), r(c), r(d), r(x), r(y))
                    }
                };
                out.push_str(&part);
            }
            out.push('Z');
        }
        out
    }

    /// 选中几笔的外框（左、上、右、下），控制点也算在内，比实际墨迹略大一点不碍事。
    pub fn bounds(&self, selected: Option<&[usize]>) -> [i32; 4] {
        let mut b = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
        let mut add = |x: f32, y: f32| {
            b = [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)];
        };
        for (i, contour) in self.contours.iter().enumerate() {
            if selected.is_some_and(|keep| !keep.contains(&i)) {
                continue;
            }
            for segment in contour {
                match *segment {
                    Segment::Move(x, y) | Segment::Line(x, y) => add(x, y),
                    Segment::Quad(a, c, x, y) => {
                        add(a, c);
                        add(x, y);
                    }
                    Segment::Cubic(a, c, d, e, x, y) => {
                        add(a, c);
                        add(d, e);
                        add(x, y);
                    }
                }
            }
        }
        b.map(r)
    }

    /// 一笔的第一个点，编号图把号码标在这里。
    pub fn start(&self, contour: usize) -> (f32, f32) {
        match self.contours[contour].first() {
            Some(Segment::Move(x, y)) => (*x, *y),
            _ => (0.0, 0.0),
        }
    }
}

fn r(value: f32) -> i32 {
    value.round() as i32
}

/// 收轮廓的笔：`move_to` 开一笔，y 翻成向下。
#[derive(Default)]
struct Pen {
    contours: Vec<Vec<Segment>>,
}

impl Pen {
    fn push(&mut self, segment: Segment) {
        if let Some(contour) = self.contours.last_mut() {
            contour.push(segment);
        }
    }
}

fn flip(y: f32) -> f32 {
    FACE_TOP - y
}

impl OutlinePen for Pen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.contours.push(vec![Segment::Move(x, flip(y))]);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.push(Segment::Line(x, flip(y)));
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.push(Segment::Quad(cx0, flip(cy0), x, flip(y)));
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.push(Segment::Cubic(cx0, flip(cy0), cx1, flip(cy1), x, flip(y)));
    }

    fn close(&mut self) {}
}
