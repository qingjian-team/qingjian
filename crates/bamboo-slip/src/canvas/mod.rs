//! 位图画布：tiny-skia `Pixmap` 之上的几个填充原语，加字形位图的逐像素 source-over 混合。坐标一律是像素、左上角原点。
//! 可以压一层层裁剪区（[`Canvas::push_clip`]），之后所有画的操作都只落在里面。

mod clip;

use std::borrow::Cow;

use clip::Clip;
use tiny_skia::{
    BlendMode, FillRule, Mask, Paint, Path, PathBuilder, Pixmap, PixmapPaint, PremultipliedColorU8,
    Rect, Shader, Stroke, Transform,
};

use crate::color::{Color, mul_u8, premultiply};
use crate::error::RenderError;

pub struct Canvas {
    /// 预乘 RGBA。
    pixmap: Pixmap,

    /// 裁剪区栈，最后一个是当前的（已与外层取交集）。
    clips: Vec<Clip>,
}

impl Canvas {
    pub fn new(width: u32, height: u32) -> Result<Self, RenderError> {
        Pixmap::new(width, height)
            .map(|pixmap| Self {
                pixmap,
                clips: Vec::new(),
            })
            .ok_or(RenderError::InvalidSize { width, height })
    }

    pub fn from_pixmap(pixmap: Pixmap) -> Self {
        Self {
            pixmap,
            clips: Vec::new(),
        }
    }

    /// 之后画的只落在这个（圆角）矩形里，与外层裁剪区取交集；与 [`Self::pop_clip`] 成对用。
    pub fn push_clip(&mut self, x: f32, y: f32, width: f32, height: f32, radius: f32) {
        let size = (self.width(), self.height());
        let (width, height) = (width.max(0.0), height.max(0.0));
        let shaped = radius > 0.0 || [x, y, width, height].iter().any(|v| v.fract() != 0.0);
        let empty = width <= 0.0 || height <= 0.0;
        let bounds = if empty {
            (0, 0, 0, 0)
        } else {
            (
                x.floor() as i32,
                y.floor() as i32,
                (x + width).ceil() as i32,
                (y + height).ceil() as i32,
            )
        };
        // 圆角只影响四个角，安全区按半径往里收（宽一点没关系，只是多建一次遮罩）
        let inset = radius.min(width / 2.0).min(height / 2.0);
        let safe = if shaped {
            (
                x.ceil() + inset,
                y.ceil() + inset,
                (x + width).floor() - inset,
                (y + height).floor() - inset,
            )
        } else {
            (x, y, x + width, y + height)
        };
        let path = round_rect(x, y, width, height, radius)
            .or_else(|| Rect::from_xywh(-2.0, -2.0, 1.0, 1.0).map(PathBuilder::from_rect));
        if let Some(path) = path {
            let clip = Clip::new(self.clips.last(), path, bounds, safe, shaped, size);
            self.clips.push(clip);
        }
    }

    pub fn pop_clip(&mut self) {
        self.clips.pop();
    }

    /// `(x, y, width, height)` 这块有没有落进当前裁剪区（没有裁剪时总是有）：整个在外面的东西不用画。
    pub fn visible(&self, x: f32, y: f32, width: f32, height: f32) -> bool {
        self.clips
            .last()
            .is_none_or(|clip| clip.needs((x, y, x + width, y + height)).is_some())
    }

    /// 当前能画的整数像素范围（左、上、右、下，右下不含）。
    fn bounds(&self) -> (i32, i32, i32, i32) {
        let full = (
            0,
            0,
            self.pixmap.width() as i32,
            self.pixmap.height() as i32,
        );
        match self.clips.last() {
            Some(clip) => (
                clip.bounds.0.max(0),
                clip.bounds.1.max(0),
                clip.bounds.2.min(full.2),
                clip.bounds.3.min(full.3),
            ),
            None => full,
        }
    }

    /// 裁剪区里 `(x, y)` 处的覆盖率；没有裁剪或矩形裁剪时为 255（范围外的已在调用方按 [`Self::bounds`] 裁掉）。
    fn coverage(&self, x: i32, y: i32) -> u8 {
        match self.clips.last() {
            Some(clip) if clip.shaped => clip.coverage(x, y),
            _ => 255,
        }
    }

    pub fn width(&self) -> u32 {
        self.pixmap.width()
    }

    pub fn height(&self) -> u32 {
        self.pixmap.height()
    }

    pub fn into_pixmap(self) -> Pixmap {
        self.pixmap
    }

    pub fn fill_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: Color) {
        let Some(rect) = Rect::from_xywh(x, y, width, height) else {
            return;
        };
        let Some(mask) = clip_mask(
            &self.clips,
            None,
            Some((x - 1.0, y - 1.0, x + width + 1.0, y + height + 1.0)),
        ) else {
            return;
        };
        self.pixmap.fill_rect(
            rect,
            &paint(color, BlendMode::SourceOver),
            Transform::identity(),
            mask.as_deref(),
        );
    }

    pub fn fill_round_rect(
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

    pub fn fill_path(&mut self, path: &Path, color: Color, blend: BlendMode) {
        let Some(mask) = clip_mask(&self.clips, None, Some(path_edges(path, 1.0))) else {
            return;
        };
        self.pixmap.fill_path(
            path,
            &paint(color, blend),
            FillRule::Winding,
            Transform::identity(),
            mask.as_deref(),
        );
    }

    /// 按现成的画笔填路径（着色器、抗锯齿都由调用方定），可带遮罩。
    pub fn fill_path_paint(&mut self, path: &Path, paint: &Paint, mask: Option<&Mask>) {
        let Some(mask) = clip_mask(&self.clips, mask, Some(path_edges(path, 1.0))) else {
            return;
        };
        self.pixmap.fill_path(
            path,
            paint,
            FillRule::Winding,
            Transform::identity(),
            mask.as_deref(),
        );
    }

    /// 按着色器（渐变、图片）填路径，可带遮罩（圆角裁切）。
    pub fn fill_path_with(&mut self, path: &Path, shader: Shader, mask: Option<&Mask>) {
        let paint = Paint {
            shader,
            anti_alias: true,
            ..Paint::default()
        };
        let Some(mask) = clip_mask(&self.clips, mask, Some(path_edges(path, 1.0))) else {
            return;
        };
        self.pixmap.fill_path(
            path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            mask.as_deref(),
        );
    }

    /// 描路径（线宽像素）。
    pub fn stroke_path(&mut self, path: &Path, width: f32, color: Color) {
        let stroke = Stroke {
            width,
            ..Stroke::default()
        };
        let Some(mask) = clip_mask(&self.clips, None, Some(path_edges(path, width / 2.0 + 1.0)))
        else {
            return;
        };
        self.pixmap.stroke_path(
            path,
            &paint(color, BlendMode::SourceOver),
            &stroke,
            Transform::identity(),
            mask.as_deref(),
        );
    }

    /// 描字形轮廓：圆角接头与端点，描边不出尖角。
    pub fn stroke_outline(&mut self, path: &Path, width: f32, color: Color) {
        let stroke = Stroke {
            width,
            line_join: tiny_skia::LineJoin::Round,
            line_cap: tiny_skia::LineCap::Round,
            ..Stroke::default()
        };
        let Some(mask) = clip_mask(&self.clips, None, Some(path_edges(path, width / 2.0 + 1.0)))
        else {
            return;
        };
        self.pixmap.stroke_path(
            path,
            &paint(color, BlendMode::SourceOver),
            &stroke,
            Transform::identity(),
            mask.as_deref(),
        );
    }

    /// 与画布同大的遮罩，`path` 里面为不透明。
    pub fn mask(&self, path: &Path) -> Option<Mask> {
        let mut mask = Mask::new(self.width(), self.height())?;
        mask.fill_path(path, FillRule::Winding, true, Transform::identity());
        Some(mask)
    }

    /// 把同大的离屏图层按不透明度合成上来。
    pub fn draw_layer(&mut self, layer: &Pixmap, opacity: f32) {
        self.draw_layer_at(0, 0, layer, opacity);
    }

    /// 把离屏图层按不透明度合成到 `(x, y)`（图层左上角）。
    pub fn draw_layer_at(&mut self, x: i32, y: i32, layer: &Pixmap, opacity: f32) {
        let paint = PixmapPaint {
            opacity,
            ..PixmapPaint::default()
        };
        let edges = (
            x as f32,
            y as f32,
            (x + layer.width() as i32) as f32,
            (y + layer.height() as i32) as f32,
        );
        let Some(mask) = clip_mask(&self.clips, None, Some(edges)) else {
            return;
        };
        self.pixmap.draw_pixmap(
            x,
            y,
            layer.as_ref(),
            &paint,
            Transform::identity(),
            mask.as_deref(),
        );
    }

    /// 另一张位图原样盖到 `(x, y)`（替换，不混合）；局部重画的结果写回整帧用。只认裁剪区的矩形范围。
    pub fn replace(&mut self, x: i32, y: i32, other: &Pixmap) {
        let (left, top, right, bottom) = self.bounds();
        let canvas_width = self.pixmap.width() as usize;
        let width = other.width() as i32;
        let col_start = (left - x).max(0);
        let col_end = width.min(right - x);
        if col_start >= col_end {
            return;
        }
        let source = other.pixels();
        let pixels = self.pixmap.pixels_mut();
        for row in 0..other.height() as i32 {
            let py = y + row;
            if py < top || py >= bottom {
                continue;
            }
            let from = row as usize * width as usize;
            let to = py as usize * canvas_width;
            pixels[to + (x + col_start) as usize..to + (x + col_end) as usize]
                .copy_from_slice(&source[from + col_start as usize..from + col_end as usize]);
        }
    }

    /// 另一张位图按变换叠上来（双线性采样，整数平移时 tiny-skia 自己退成最近邻）；循环动画的姿态用。
    pub fn draw_transformed(&mut self, layer: &Pixmap, transform: Transform, opacity: f32) {
        let paint = PixmapPaint {
            opacity,
            quality: tiny_skia::FilterQuality::Bilinear,
            ..PixmapPaint::default()
        };
        let Some(mask) = clip_mask(&self.clips, None, None) else {
            return;
        };
        self.pixmap
            .draw_pixmap(0, 0, layer.as_ref(), &paint, transform, mask.as_deref());
    }

    /// 把另一张位图整张叠上来（左上角对齐到 `(x, y)`）。
    pub fn blend_pixmap(&mut self, x: i32, y: i32, other: &Pixmap) {
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
    pub fn blend_mask(
        &mut self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        data: &[u8],
        color: Color,
    ) {
        // 按行裁到画布（与裁剪区）内，逐像素的算法与 blend_pixel 相同（结果逐位一致），只是省掉每个像素的越界检查
        let (left, top, right, bottom) = self.bounds();
        let canvas_width = self.pixmap.width() as usize;
        let col_start = (left - x).max(0);
        let col_end = (width as i32).min(right - x);
        if col_start >= col_end {
            return;
        }
        // 覆盖率 → 预乘颜色查表（同 Skia 的 A8 遮罩），每像素只剩一次查表加一次混合
        let table: Vec<PremultipliedColorU8> =
            (0..=255u8).map(|c| color.premultiplied(c)).collect();
        let shaped = self.clips.last().filter(|clip| clip.shaped);
        let pixels = self.pixmap.pixels_mut();
        for row in 0..height as i32 {
            let py = y + row;
            if py < top || py >= bottom {
                continue;
            }
            let Some(line) =
                data.get((row as usize * width as usize)..((row as usize + 1) * width as usize))
            else {
                return;
            };
            let base = py as usize * canvas_width;
            for col in col_start..col_end {
                let mut coverage = line[col as usize];
                if let Some(clip) = shaped {
                    coverage = mul_u8(coverage, clip.coverage(x + col, py));
                }
                if coverage == 0 {
                    continue;
                }
                let dst = &mut pixels[base + (x + col) as usize];
                *dst = source_over(table[coverage as usize], *dst);
            }
        }
    }

    /// 直通 RGBA 位图（彩色 emoji）叠上来。
    pub fn blend_rgba(&mut self, x: i32, y: i32, width: u32, height: u32, data: &[u8]) {
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

    /// source-over：`dst = src + dst × (1 − src.a)`，裁剪区外的不画。
    fn blend_pixel(&mut self, x: i32, y: i32, src: PremultipliedColorU8) {
        let (left, top, right, bottom) = self.bounds();
        if x < left || y < top || x >= right || y >= bottom {
            return;
        }
        let src = scale(src, self.coverage(x, y));
        let index = y as usize * self.pixmap.width() as usize + x as usize;
        let dst = &mut self.pixmap.pixels_mut()[index];
        *dst = source_over(src, *dst);
    }

    /// 另一张位图按整数偏移叠上来（source-over）；缓存的画面贴回画布用。
    /// 逐像素整数运算：不透明的直接拷、透明的跳过（tiny-skia 的 `draw_pixmap` 没有拷贝快路径，每像素走一遍高精度采样）。
    pub fn composite(&mut self, x: i32, y: i32, other: &Pixmap) {
        let (left, top, right, bottom) = self.bounds();
        let canvas_width = self.pixmap.width() as usize;
        let width = other.width() as i32;
        let col_start = (left - x).max(0);
        let col_end = width.min(right - x);
        if col_start >= col_end {
            return;
        }
        let shaped = self.clips.last().filter(|clip| clip.shaped);
        let source = other.pixels();
        let pixels = self.pixmap.pixels_mut();
        for row in 0..other.height() as i32 {
            let py = y + row;
            if py < top || py >= bottom {
                continue;
            }
            let from = row as usize * width as usize;
            let to = py as usize * canvas_width;
            for col in col_start..col_end {
                let mut src = source[from + col as usize];
                if let Some(clip) = shaped {
                    src = scale(src, clip.coverage(x + col, py));
                }
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

/// 一个操作在当前裁剪区下怎么画：`None` 整个被裁掉不用画；`Some(遮罩)` 照画，遮罩是裁剪区（要的话）与调用方自己的遮罩（圆角裁切）的交集。
/// `edges` 是操作会画到的范围，不知道时为 `None`（总带遮罩）。
fn clip_mask<'a>(
    clips: &'a [Clip],
    own: Option<&'a Mask>,
    edges: Option<clip::Edges>,
) -> Option<Option<Cow<'a, Mask>>> {
    let Some(clip) = clips.last() else {
        return Some(own.map(Cow::Borrowed));
    };
    let needs = match edges {
        Some(edges) => clip.needs(edges)?,
        None => true,
    };
    if !needs {
        return Some(own.map(Cow::Borrowed));
    }
    Some(match own {
        None => clip.mask().map(Cow::Borrowed),
        Some(own) => {
            let mut mask = own.clone();
            for path in clip.paths() {
                mask.intersect_path(path, FillRule::Winding, true, Transform::identity());
            }
            Some(Cow::Owned(mask))
        }
    })
}

/// 路径画出来的范围：外接矩形往外放 `spread`（描边半宽加抗锯齿）。
fn path_edges(path: &Path, spread: f32) -> clip::Edges {
    let bounds = path.bounds();
    (
        bounds.left() - spread,
        bounds.top() - spread,
        bounds.right() + spread,
        bounds.bottom() + spread,
    )
}

/// 预乘颜色整体乘一个覆盖率。
fn scale(color: PremultipliedColorU8, coverage: u8) -> PremultipliedColorU8 {
    if coverage == 255 {
        return color;
    }
    PremultipliedColorU8::from_rgba(
        mul_u8(color.red(), coverage),
        mul_u8(color.green(), coverage),
        mul_u8(color.blue(), coverage),
        mul_u8(color.alpha(), coverage),
    )
    .unwrap_or(color)
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
pub fn round_rect(x: f32, y: f32, width: f32, height: f32, radius: f32) -> Option<Path> {
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

    #[test]
    fn clip_limits_fills_and_glyphs_and_nests() {
        let mut canvas = Canvas::new(4, 1).unwrap();
        canvas.push_clip(1.0, 0.0, 2.0, 1.0, 0.0);
        canvas.fill_rect(0.0, 0.0, 4.0, 1.0, Color::rgb(0, 0, 255));
        canvas.push_clip(2.0, 0.0, 2.0, 1.0, 0.0);
        canvas.blend_mask(0, 0, 4, 1, &[255; 4], Color::rgb(255, 0, 0));
        canvas.pop_clip();
        canvas.pop_clip();
        let pixmap = canvas.into_pixmap();
        let colors: Vec<(u8, u8, u8)> = (0..4)
            .map(|x| {
                let p = pixmap.pixel(x, 0).unwrap();
                (p.red(), p.blue(), p.alpha())
            })
            .collect();
        assert_eq!(
            colors,
            vec![(0, 0, 0), (0, 255, 255), (255, 0, 255), (0, 0, 0)]
        );
    }
}
