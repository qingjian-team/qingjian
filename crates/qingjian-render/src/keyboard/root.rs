//! 键面上的一个字根：一段矢量轮廓（字面方块 0–1000，y 向下），从思源黑体取的，见 `assets/schemes/wubi86/README.md`。
//! 部件（虎字头、无尾鱼……）只露出一块：画的时候按露出区域裁，并放大到占满一格，不然只剩半格大。

use std::sync::Arc;

use bamboo_slip::tiny_skia::{BlendMode, Path, PathBuilder, Pixmap, Rect, Transform};
use bamboo_slip::{Canvas, Color, Icon};

/// 部件最多放大到几倍，免得一两笔的部件撑得太粗。
const MAX_ZOOM: f32 = 1.5;

/// 字面方块的边长（字体单位）。
const FACE: f32 = 1000.0;

#[derive(Debug, Clone)]
pub struct Root {
    outline: Arc<Outline>,
}

#[derive(Debug)]
struct Outline {
    /// 原样留着，比较两帧是否相同用。
    source: String,

    path: Path,

    /// 露出区域；`None` 是整字，按字面方块画。
    frame: Option<Rect>,
}

impl PartialEq for Root {
    fn eq(&self, other: &Self) -> bool {
        self.outline.source == other.outline.source && self.outline.frame == other.outline.frame
    }
}

impl Root {
    /// `svg` 只认绝对坐标的 `M L Q C Z`（工具导出的就是这几种）；`frame` 是露出区域（左、上、右、下）。解析不了返回 `None`。
    pub fn from_svg(svg: &str, frame: Option<[f32; 4]>) -> Option<Self> {
        let path = parse(svg)?;
        let frame = match frame {
            Some([left, top, right, bottom]) => Some(Rect::from_ltrb(left, top, right, bottom)?),
            None => None,
        };
        Some(Self {
            outline: Arc::new(Outline {
                source: svg.to_owned(),
                path,
                frame,
            }),
        })
    }

    /// 交给场景当图标画。
    pub(crate) fn icon(&self) -> Arc<dyn Icon> {
        Arc::new(RootIcon(Arc::clone(&self.outline)))
    }
}

#[derive(Debug)]
struct RootIcon(Arc<Outline>);

impl Icon for RootIcon {
    /// 先画在自己的小位图上、把露出区域以外擦掉再贴上去，与齿轮图标同一个做法（不动画布的裁剪栈）。
    fn draw(&self, canvas: &mut Canvas, x: f32, y: f32, size: f32, color: Color) {
        let outline = &self.0;
        let (frame, zoom) = match outline.frame {
            Some(frame) => (
                frame,
                (FACE / frame.width().max(frame.height())).min(MAX_ZOOM),
            ),
            None => (
                Rect::from_xywh(0.0, 0.0, FACE, FACE).unwrap_or(outline.path.bounds()),
                1.0,
            ),
        };
        let side = size.ceil() as u32 + 1;
        let Some(pixmap) = Pixmap::new(side, side) else {
            return;
        };
        let scale = size / FACE * zoom;
        // 露出区域的中心对准方块中心
        let dx = size / 2.0 - (frame.left() + frame.width() / 2.0) * scale;
        let dy = size / 2.0 - (frame.top() + frame.height() / 2.0) * scale;
        let Some(path) = outline
            .path
            .clone()
            .transform(Transform::from_scale(scale, scale).post_translate(dx, dy))
        else {
            return;
        };
        let mut layer = Canvas::from_pixmap(pixmap);
        layer.fill_path(&path, color, BlendMode::SourceOver);
        if outline.frame.is_some() {
            let (left, top) = (dx + frame.left() * scale, dy + frame.top() * scale);
            let (right, bottom) = (left + frame.width() * scale, top + frame.height() * scale);
            let edge = side as f32;
            for (l, t, r, b) in [
                (0.0, 0.0, edge, top),
                (0.0, bottom, edge, edge),
                (0.0, top, left, bottom),
                (right, top, edge, bottom),
            ] {
                if let Some(rect) = Rect::from_ltrb(l, t, r, b) {
                    layer.fill_path(
                        &PathBuilder::from_rect(rect),
                        Color::rgb(0, 0, 0),
                        BlendMode::Clear,
                    );
                }
            }
        }
        canvas.blend_pixmap(x.round() as i32, y.round() as i32, &layer.into_pixmap());
    }
}

/// 解析 `M x y`、`L x y`、`Q x y x y`、`C x y x y x y`、`Z`（命令紧跟数字，数字用空格隔开）。
fn parse(svg: &str) -> Option<Path> {
    let mut builder = PathBuilder::new();
    let mut rest = svg.trim();
    while let Some(command) = rest.chars().next() {
        let body_end = rest[1..]
            .find(|c: char| c.is_ascii_alphabetic())
            .map_or(rest.len(), |i| i + 1);
        let numbers: Vec<f32> = rest[1..body_end]
            .split_whitespace()
            .map(str::parse)
            .collect::<Result<_, _>>()
            .ok()?;
        match (command, numbers.as_slice()) {
            ('M', [x, y]) => builder.move_to(*x, *y),
            ('L', [x, y]) => builder.line_to(*x, *y),
            ('Q', [a, b, x, y]) => builder.quad_to(*a, *b, *x, *y),
            ('C', [a, b, c, d, x, y]) => builder.cubic_to(*a, *b, *c, *d, *x, *y),
            ('Z', []) => builder.close(),
            _ => return None,
        }
        rest = &rest[body_end..];
    }
    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::Root;

    #[test]
    fn parses_exported_commands() {
        let root = Root::from_svg("M0 0L100 0Q150 50 100 100C80 120 20 120 0 100Z", None);
        assert!(root.is_some());
        assert!(Root::from_svg("M0 0L1", None).is_none());
        assert!(Root::from_svg("M0 0A1 1 0 0 1 2 2Z", None).is_none());
    }
}
