//! 关闭按钮的叉：两条圆头斜线，颜色跟主题走。自己画是为了不受字体回退影响（✕ 在有的字体里粗细差很多）。

use bamboo_slip::tiny_skia::{BlendMode, LineCap, PathBuilder, Pixmap, Stroke};

use bamboo_slip::{Canvas, Color, Icon};

/// 线宽相对边长的比例。
const STROKE_RATIO: f32 = 0.08;

/// 叉的两臂离边多远（相对边长）。
const INSET_RATIO: f32 = 0.33;

#[derive(Debug)]
pub(super) struct Close;

impl Icon for Close {
    fn draw(&self, canvas: &mut Canvas, x: f32, y: f32, size: f32, color: Color) {
        let side = size.ceil() as u32 + 1;
        let Some(icon) = Pixmap::new(side, side) else {
            return;
        };
        let (near, far) = (size * INSET_RATIO, size * (1.0 - INSET_RATIO));
        let mut path = PathBuilder::new();
        path.move_to(near, near);
        path.line_to(far, far);
        path.move_to(far, near);
        path.line_to(near, far);
        let stroke = Stroke {
            width: (size * STROKE_RATIO).max(1.0),
            line_cap: LineCap::Round,
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
