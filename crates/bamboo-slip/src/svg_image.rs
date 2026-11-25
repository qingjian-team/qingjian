//! SVG 图片：加载时解析一次，画的时候按要的像素尺寸栅格成位图（缓存几种尺寸），之后与 PNG 走同一条贴图路径。
//!
//! SVG 的单位当作点。不读任何外部文件（`<image href="…">` 的路径一律不解析），内嵌位图不解码，`<text>` 不画：
//! 位图另用 PNG，文字导出前转成轮廓。也能当 [`Icon`] 画成单色图标（只取形状、按给的颜色上色）。

use std::sync::{Arc, Mutex};

use resvg::usvg;
use tiny_skia::{Pixmap, Transform};

use crate::canvas::Canvas;
use crate::color::Color;
use crate::scene::Icon;

/// 栅格结果最多留几种尺寸：动画中途尺寸会变，旧的按先进先出丢掉。
const RASTER_CACHE: usize = 8;

/// 栅格过的尺寸（像素宽高）与位图，新的在后。
type Rasters = Vec<((u32, u32), Arc<Pixmap>)>;

pub struct SvgImage {
    tree: usvg::Tree,

    rasters: Mutex<Rasters>,
}

impl SvgImage {
    pub fn parse(data: &[u8]) -> Result<Self, usvg::Error> {
        let options = usvg::Options {
            image_href_resolver: usvg::ImageHrefResolver {
                resolve_string: Box::new(|_, _| None),
                ..usvg::ImageHrefResolver::default()
            },
            ..usvg::Options::default()
        };
        Ok(Self {
            tree: usvg::Tree::from_data(data, &options)?,
            rasters: Mutex::new(Vec::new()),
        })
    }

    /// 原始尺寸（SVG 的宽高，当作点）。
    pub fn size(&self) -> (f32, f32) {
        let size = self.tree.size();
        (size.width(), size.height())
    }

    /// 整张拉伸到 `width` × `height` 像素的位图；尺寸为 0 时为 `None`。
    pub fn raster(&self, width: u32, height: u32) -> Option<Arc<Pixmap>> {
        let mut rasters = self
            .rasters
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if let Some((_, pixmap)) = rasters.iter().find(|(size, _)| *size == (width, height)) {
            return Some(Arc::clone(pixmap));
        }
        let mut pixmap = Pixmap::new(width, height)?;
        let (svg_width, svg_height) = self.size();
        let transform = Transform::from_scale(width as f32 / svg_width, height as f32 / svg_height);
        resvg::render(&self.tree, transform, &mut pixmap.as_mut());
        let pixmap = Arc::new(pixmap);
        if rasters.len() >= RASTER_CACHE {
            rasters.remove(0);
        }
        rasters.push(((width, height), Arc::clone(&pixmap)));
        Some(pixmap)
    }
}

/// 当单色图标画：只取形状（透明度），按给的颜色上色，深浅色外观共用一份 SVG。
impl Icon for SvgImage {
    fn draw(&self, canvas: &mut Canvas, x: f32, y: f32, size: f32, color: Color) {
        let side = size.round() as u32;
        let Some(pixmap) = self.raster(side, side) else {
            return;
        };
        let coverage: Vec<u8> = pixmap.pixels().iter().map(|pixel| pixel.alpha()).collect();
        canvas.blend_mask(
            x.round() as i32,
            y.round() as i32,
            side,
            side,
            &coverage,
            color,
        );
    }
}

impl std::fmt::Debug for SvgImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SvgImage")
            .field("size", &self.size())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::SvgImage;

    const SQUARE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="20">
        <rect width="10" height="20" fill="#ff0000"/>
    </svg>"##;

    #[test]
    fn rasters_at_any_size_and_caches() {
        let svg = SvgImage::parse(SQUARE.as_bytes()).unwrap();
        assert_eq!(svg.size(), (10.0, 20.0));
        let big = svg.raster(30, 60).unwrap();
        assert_eq!((big.width(), big.height()), (30, 60));
        let center = big.pixel(15, 30).unwrap();
        assert_eq!((center.red(), center.alpha()), (255, 255));
        assert!(std::sync::Arc::ptr_eq(&big, &svg.raster(30, 60).unwrap()));
        assert!(svg.raster(0, 10).is_none());
    }

    #[test]
    fn external_files_are_not_read() {
        // 引用磁盘上真实存在的红色 SVG（SVG 不需要位图解码，读了就画得出来）：不读，画出来什么都没有
        let outside = std::env::temp_dir().join("qingjian-svg-outside.svg");
        std::fs::write(&outside, SQUARE).unwrap();
        let source = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="10" height="10">
                <image width="10" height="10" xlink:href="{}"/>
            </svg>"#,
            outside.display()
        );
        let svg = SvgImage::parse(source.as_bytes()).unwrap();
        let pixmap = svg.raster(10, 10).unwrap();
        assert!(pixmap.pixels().iter().all(|pixel| pixel.alpha() == 0));
    }

    #[test]
    fn as_icon_keeps_the_shape_and_takes_the_given_color() {
        use crate::canvas::Canvas;
        use crate::color::Color;
        use crate::scene::Icon;

        let svg = SvgImage::parse(SQUARE.as_bytes()).unwrap();
        let mut canvas = Canvas::new(40, 40).unwrap();
        svg.draw(&mut canvas, 5.0, 5.0, 20.0, Color::rgb(0, 0, 255));
        let pixmap = canvas.into_pixmap();
        let inside = pixmap.pixel(10, 15).unwrap();
        assert_eq!((inside.red(), inside.blue(), inside.alpha()), (0, 255, 255));
        assert_eq!(pixmap.pixel(30, 30).unwrap().alpha(), 0);
    }

    #[test]
    fn broken_svg_is_an_error() {
        assert!(SvgImage::parse(b"<svg").is_err());
    }
}
