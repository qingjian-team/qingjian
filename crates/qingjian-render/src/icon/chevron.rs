//! 翻页箭头：一个圆头圆角的尖括号，`chevron-left` / `chevron-right`，颜色跟主题走（图片箭头改不了色）。

use bamboo_slip::tiny_skia::{BlendMode, LineCap, LineJoin, PathBuilder, Pixmap, Stroke};

use bamboo_slip::{Canvas, Color, Icon};

/// 线宽相对边长的比例。
const STROKE_RATIO: f32 = 0.13;

/// 尖括号的宽度（两臂末端到尖端的横向距离）相对边长的比例；高是它的两倍。
const ARM_RATIO: f32 = 0.22;

#[derive(Debug)]
pub(super) struct Chevron {
    /// 尖朝右（下一页）。
    pub(super) right: bool,
}

impl Icon for Chevron {
    fn draw(&self, canvas: &mut Canvas, x: f32, y: f32, size: f32, color: Color) {
        let side = size.ceil() as u32 + 1;
        let Some(icon) = Pixmap::new(side, side) else {
            return;
        };
        let center = size / 2.0;
        let arm = size * ARM_RATIO;
        let (tip, ends) = if self.right {
            (center + arm / 2.0, center - arm / 2.0)
        } else {
            (center - arm / 2.0, center + arm / 2.0)
        };
        let mut path = PathBuilder::new();
        path.move_to(ends, center - arm);
        path.line_to(tip, center);
        path.line_to(ends, center + arm);
        let stroke = Stroke {
            width: (size * STROKE_RATIO).max(1.0),
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Stroke::default()
        };
        let Some(outline) = path.finish().and_then(|path| path.stroke(&stroke, 1.0)) else {
            return;
        };
        let mut layer = Canvas::from_pixmap(icon);
        layer.fill_path(&outline, color, BlendMode::SourceOver);
        canvas.blend_pixmap(x.round() as i32, y.round() as i32, &layer.into_pixmap());
    }
}
