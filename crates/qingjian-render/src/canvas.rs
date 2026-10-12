//! 位图画布：tiny-skia `Pixmap` 之上的几个填充原语，加字形位图的逐像素 source-over 混合。坐标一律是像素、左上角原点。

use tiny_skia::{
    BlendMode, FillRule, Mask, Paint, Path, PathBuilder, Pixmap, PixmapPaint, PremultipliedColorU8,
    Rect, Shader, Stroke, Transform,
};

use crate::color::{Color, mul_u8, premultiply};
use crate::error::RenderError;

pub(crate) struct Canvas {
    /// 预乘 RGBA。
    pixmap: Pixmap,
}

impl Canvas {
    pub(crate) fn new(width: u32, height: u32) -> Result<Self, RenderError> {
        Pixmap::new(width, height)
            .map(|pixmap| Self { pixmap })
            .ok_or(RenderError::InvalidSize { width, height })
    }

    pub(crate) fn from_pixmap(pixmap: Pixmap) -> Self {
        Self { pixmap }
    }

    pub(crate) fn width(&self) -> u32 {
        self.pixmap.width()
    }

    pub(crate) fn height(&self) -> u32 {
        self.pixmap.height()
    }

    pub(crate) fn into_pixmap(self) -> Pixmap {
        self.pixmap
    }

    pub(crate) fn fill_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: Color) {
        let Some(rect) = Rect::from_xywh(x, y, width, height) else {
            return;
        };
        self.pixmap.fill_rect(
            rect,
            &paint(color, BlendMode::SourceOver),
            Transform::identity(),
            None,
        );
    }

    pub(crate) fn fill_round_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        radius: f32,
        color: Color,
    ) {
        if let Some(path) = round_rect(x, y, width, height, radius) {
            self.fill_path(&path, color, BlendMode::SourceOver);
        }
    }

    pub(crate) fn fill_path(&mut self, path: &Path, color: Color, blend: BlendMode) {
        self.pixmap.fill_path(
            path,
            &paint(color, blend),
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }

    /// 按现成的画笔填路径（着色器、抗锯齿都由调用方定），可带遮罩。
    pub(crate) fn fill_path_paint(&mut self, path: &Path, paint: &Paint, mask: Option<&Mask>) {
        self.pixmap
            .fill_path(path, paint, FillRule::Winding, Transform::identity(), mask);
    }

    /// 按着色器（渐变、图片）填路径，可带遮罩（圆角裁切）。
    pub(crate) fn fill_path_with(&mut self, path: &Path, shader: Shader, mask: Option<&Mask>) {
        let paint = Paint {
            shader,
            anti_alias: true,
            ..Paint::default()
        };
        self.pixmap
            .fill_path(path, &paint, FillRule::Winding, Transform::identity(), mask);
    }

    /// 描路径（线宽像素）。
    pub(crate) fn stroke_path(&mut self, path: &Path, width: f32, color: Color) {
        let stroke = Stroke {
            width,
            ..Stroke::default()
        };
        self.pixmap.stroke_path(
            path,
            &paint(color, BlendMode::SourceOver),
            &stroke,
            Transform::identity(),
            None,
        );
    }

    /// 描字形轮廓：圆角接头与端点，描边不出尖角。
    pub(crate) fn stroke_outline(&mut self, path: &Path, width: f32, color: Color) {
        let stroke = Stroke {
            width,
            line_join: tiny_skia::LineJoin::Round,
            line_cap: tiny_skia::LineCap::Round,
            ..Stroke::default()
        };
        self.pixmap.stroke_path(
            path,
            &paint(color, BlendMode::SourceOver),
            &stroke,
            Transform::identity(),
            None,
        );
    }

    /// 与画布同大的遮罩，`path` 里面为不透明。
    pub(crate) fn mask(&self, path: &Path) -> Option<Mask> {
        let mut mask = Mask::new(self.width(), self.height())?;
        mask.fill_path(path, FillRule::Winding, true, Transform::identity());
        Some(mask)
    }

    /// 把同大的离屏图层按不透明度合成上来。
    pub(crate) fn draw_layer(&mut self, layer: &Pixmap, opacity: f32) {
        let paint = PixmapPaint {
            opacity,
            ..PixmapPaint::default()
        };
        self.pixmap
            .draw_pixmap(0, 0, layer.as_ref(), &paint, Transform::identity(), None);
    }

    /// 另一张位图原样盖到 `(x, y)`（替换，不混合）；局部重画的结果写回整帧用。
    pub(crate) fn replace(&mut self, x: i32, y: i32, other: &Pixmap) {
        let canvas_width = self.pixmap.width() as i32;
        let canvas_height = self.pixmap.height() as i32;
        let width = other.width() as i32;
        let col_start = (-x).max(0);
        let col_end = width.min(canvas_width - x);
        if col_start >= col_end {
            return;
        }
        let source = other.pixels();
        let pixels = self.pixmap.pixels_mut();
        for row in 0..other.height() as i32 {
            let py = y + row;
            if py < 0 || py >= canvas_height {
                continue;
            }
            let from = row as usize * width as usize;
            let to = py as usize * canvas_width as usize;
            pixels[to + (x + col_start) as usize..to + (x + col_end) as usize]
                .copy_from_slice(&source[from + col_start as usize..from + col_end as usize]);
        }
    }

    /// 另一张位图按变换叠上来（双线性采样，整数平移时 tiny-skia 自己退成最近邻）；循环动画的姿态用。
    pub(crate) fn draw_transformed(&mut self, layer: &Pixmap, transform: Transform, opacity: f32) {
        let paint = PixmapPaint {
            opacity,
            quality: tiny_skia::FilterQuality::Bilinear,
            ..PixmapPaint::default()
        };
        self.pixmap
            .draw_pixmap(0, 0, layer.as_ref(), &paint, transform, None);
    }

    /// 把另一张位图整张叠上来（左上角对齐到 `(x, y)`）。
    pub(crate) fn blend_pixmap(&mut self, x: i32, y: i32, other: &Pixmap) {
        let width = other.width();
        for (i, pixel) in other.pixels().iter().enumerate() {
            if pixel.alpha() == 0 {
                continue;
            }
            let px = x + (i as u32 % width) as i32;
            let py = y + (i as u32 / width) as i32;
            self.blend_pixel(px, py, *pixel);
        }
    }

    /// 8 位覆盖率遮罩（普通字形）按颜色叠上来。
    pub(crate) fn blend_mask(
        &mut self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        data: &[u8],
        color: Color,
    ) {
        // 按行裁到画布内，逐像素的算法与 blend_pixel 相同（结果逐位一致），只是省掉每个像素的越界检查
        let canvas_width = self.pixmap.width() as i32;
        let canvas_height = self.pixmap.height() as i32;
        let col_start = (-x).max(0);
        let col_end = (width as i32).min(canvas_width - x);
        if col_start >= col_end {
            return;
        }
        // 覆盖率 → 预乘颜色查表（同 Skia 的 A8 遮罩），每像素只剩一次查表加一次混合
        let table: Vec<PremultipliedColorU8> =
            (0..=255u8).map(|c| color.premultiplied(c)).collect();
        let pixels = self.pixmap.pixels_mut();
        for row in 0..height as i32 {
            let py = y + row;
            if py < 0 || py >= canvas_height {
                continue;
            }
            let Some(line) =
                data.get((row as usize * width as usize)..((row as usize + 1) * width as usize))
            else {
                return;
            };
            let base = py as usize * canvas_width as usize;
            for col in col_start..col_end {
                let coverage = line[col as usize];
                if coverage == 0 {
                    continue;
                }
                let dst = &mut pixels[base + (x + col) as usize];
                *dst = source_over(table[coverage as usize], *dst);
            }
        }
    }

    /// 直通 RGBA 位图（彩色 emoji）叠上来。
    pub(crate) fn blend_rgba(&mut self, x: i32, y: i32, width: u32, height: u32, data: &[u8]) {
        for (i, px) in data.chunks_exact(4).enumerate() {
            if px[3] == 0 {
                continue;
            }
            let col = (i as u32 % width) as i32;
            let row = (i as u32 / width) as i32;
            if row >= height as i32 {
                return;
            }
            self.blend_pixel(x + col, y + row, premultiply(px[0], px[1], px[2], px[3]));
        }
    }

    /// source-over：`dst = src + dst × (1 − src.a)`。
    fn blend_pixel(&mut self, x: i32, y: i32, src: PremultipliedColorU8) {
        if x < 0 || y < 0 || x >= self.pixmap.width() as i32 || y >= self.pixmap.height() as i32 {
            return;
        }
        let index = y as usize * self.pixmap.width() as usize + x as usize;
        let dst = &mut self.pixmap.pixels_mut()[index];
        *dst = source_over(src, *dst);
    }

    /// 另一张位图按整数偏移叠上来（source-over）；缓存的画面贴回画布用。
    /// 逐像素整数运算：不透明的直接拷、透明的跳过（tiny-skia 的 `draw_pixmap` 没有拷贝快路径，每像素走一遍高精度采样）。
    pub(crate) fn composite(&mut self, x: i32, y: i32, other: &Pixmap) {
        let canvas_width = self.pixmap.width() as i32;
        let canvas_height = self.pixmap.height() as i32;
        let width = other.width() as i32;
        let col_start = (-x).max(0);
        let col_end = width.min(canvas_width - x);
        if col_start >= col_end {
            return;
        }
        let source = other.pixels();
        let pixels = self.pixmap.pixels_mut();
        for row in 0..other.height() as i32 {
            let py = y + row;
            if py < 0 || py >= canvas_height {
                continue;
            }
            let from = row as usize * width as usize;
            let to = py as usize * canvas_width as usize;
            for col in col_start..col_end {
                let src = source[from + col as usize];
                let dst = &mut pixels[to + (x + col) as usize];
                match src.alpha() {
                    0 => {}
                    255 => *dst = src,
                    _ => *dst = source_over(src, *dst),
                }
            }
        }
    }
}

/// source-over：`dst = src + dst × (1 − src.a)`，各通道不超过 alpha（保持预乘合法）。
fn source_over(src: PremultipliedColorU8, dst: PremultipliedColorU8) -> PremultipliedColorU8 {
    let inverse = 255 - src.alpha();
    let a = src.alpha().saturating_add(mul_u8(dst.alpha(), inverse));
    let channel = |s: u8, d: u8| s.saturating_add(mul_u8(d, inverse)).min(a);
    let r = channel(src.red(), dst.red());
    let g = channel(src.green(), dst.green());
    let b = channel(src.blue(), dst.blue());
    PremultipliedColorU8::from_rgba(r, g, b, a).unwrap_or(dst)
}

fn paint(color: Color, blend: BlendMode) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color(color.to_skia());
    paint.anti_alias = true;
    paint.blend_mode = blend;
    paint
}

/// 圆角矩形路径；圆角用三次贝塞尔近似四分之一圆。
pub(crate) fn round_rect(x: f32, y: f32, width: f32, height: f32, radius: f32) -> Option<Path> {
    let r = radius.min(width / 2.0).min(height / 2.0).max(0.0);
    if r <= 0.0 {
        return Rect::from_xywh(x, y, width, height).map(PathBuilder::from_rect);
    }
    // 四分之一圆的贝塞尔控制点系数
    const KAPPA: f32 = 0.552_284_8;
    let k = r * KAPPA;
    let (right, bottom) = (x + width, y + height);
    let mut path = PathBuilder::new();
    path.move_to(x + r, y);
    path.line_to(right - r, y);
    path.cubic_to(right - r + k, y, right, y + r - k, right, y + r);
    path.line_to(right, bottom - r);
    path.cubic_to(
        right,
        bottom - r + k,
        right - r + k,
        bottom,
        right - r,
        bottom,
    );
    path.line_to(x + r, bottom);
    path.cubic_to(x + r - k, bottom, x, bottom - r + k, x, bottom - r);
    path.line_to(x, y + r);
    path.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    path.close();
    path.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blends_source_over_and_clips_to_bounds() {
        let mut canvas = Canvas::new(2, 1).unwrap();
        canvas.fill_rect(0.0, 0.0, 2.0, 1.0, Color::rgb(0, 0, 255));
        canvas.blend_mask(0, 0, 1, 1, &[255], Color::rgb(255, 0, 0));
        canvas.blend_mask(5, 5, 1, 1, &[255], Color::rgb(255, 0, 0));
        let pixmap = canvas.into_pixmap();
        let red = pixmap.pixel(0, 0).unwrap();
        assert_eq!((red.red(), red.blue(), red.alpha()), (255, 0, 255));
        let blue = pixmap.pixel(1, 0).unwrap();
        assert_eq!((blue.red(), blue.blue()), (0, 255));
    }

    #[test]
    fn half_alpha_over_white_lightens() {
        let mut canvas = Canvas::new(1, 1).unwrap();
        canvas.fill_rect(0.0, 0.0, 1.0, 1.0, Color::rgb(255, 255, 255));
        canvas.blend_mask(0, 0, 1, 1, &[255], Color::gray(0, 128));
        let px = canvas.into_pixmap().pixel(0, 0).unwrap();
        assert_eq!(px.alpha(), 255);
        assert!((px.red() as i32 - 127).abs() <= 1, "got {}", px.red());
    }
}
