//! 图片的转换：系统剪贴板给的 RGBA 与存储用的 PNG、卡片用的缩略图互转。

use bamboo_slip::tiny_skia::{ColorU8, FilterQuality, IntSize, Pixmap, PixmapPaint, Transform};
use qingjian_clipboard::MAX_IMAGE_BYTES;

/// 缩略图的长边（像素）。
const THUMBNAIL_SIDE: u32 = 640;

/// 再大的图不记（4000 万像素约 160 MB 未压缩，编码会卡住监听线程）。
const MAX_PIXELS: u64 = 40_000_000;

/// 编成 PNG 与缩略图的 PNG；图太大或编码失败为 `None`。
pub fn encode(width: u32, height: u32, rgba: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        tracing::info!(width, height, "image too large to record");
        return None;
    }
    let mut pixmap = Pixmap::new(width, height)?;
    for (out, source) in pixmap.pixels_mut().iter_mut().zip(rgba.chunks_exact(4)) {
        *out = ColorU8::from_rgba(source[0], source[1], source[2], source[3]).premultiply();
    }
    let png = pixmap.encode_png().ok()?;
    if png.len() > MAX_IMAGE_BYTES {
        tracing::info!(size = png.len(), "image too large to record");
        return None;
    }
    let thumbnail = thumbnail(&pixmap)?.encode_png().ok()?;
    Some((png, thumbnail))
}

fn thumbnail(source: &Pixmap) -> Option<Pixmap> {
    let side = source.width().max(source.height());
    let scale = (THUMBNAIL_SIDE as f32 / side as f32).min(1.0);
    let size = IntSize::from_wh(
        ((source.width() as f32 * scale) as u32).max(1),
        ((source.height() as f32 * scale) as u32).max(1),
    )?;
    let mut out = Pixmap::new(size.width(), size.height())?;
    let paint = PixmapPaint {
        quality: FilterQuality::Bilinear,
        ..PixmapPaint::default()
    };
    out.draw_pixmap(
        0,
        0,
        source.as_ref(),
        &paint,
        Transform::from_scale(scale, scale),
        None,
    );
    Some(out)
}

/// 别的设备发来的图片文件转成 PNG 与缩略图。本来就是 PNG 的不重新编码：字节变了同步 id 就对不上了。
pub fn from_file(bytes: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    let decoded = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (width, height) = decoded.dimensions();
    let (png, thumbnail) = encode(width, height, decoded.as_raw())?;
    let is_png = image::guess_format(bytes).ok() == Some(image::ImageFormat::Png);
    Some((if is_png { bytes.to_vec() } else { png }, thumbnail))
}

/// PNG 解回 `(宽, 高, RGBA)`，写回系统剪贴板用。
pub fn decode(png: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let pixmap = Pixmap::decode_png(png).ok()?;
    let mut rgba = Vec::with_capacity(pixmap.data().len());
    for pixel in pixmap.pixels() {
        let color = pixel.demultiply();
        rgba.extend_from_slice(&[color.red(), color.green(), color.blue(), color.alpha()]);
    }
    Some((pixmap.width(), pixmap.height(), rgba))
}

#[cfg(test)]
mod tests {
    use super::{decode, encode};

    #[test]
    fn png_round_trip_keeps_opaque_pixels() {
        let rgba = [10, 20, 30, 255, 200, 100, 50, 255];
        let (png, thumbnail) = encode(2, 1, &rgba).unwrap();
        assert_eq!(decode(&png).unwrap(), (2, 1, rgba.to_vec()));
        assert_eq!(decode(&thumbnail).unwrap().0, 2);
    }

    #[test]
    fn thumbnail_shrinks_the_long_side() {
        let rgba = vec![255; 2000 * 100 * 4];
        let (_, thumbnail) = encode(2000, 100, &rgba).unwrap();
        let (width, height, _) = decode(&thumbnail).unwrap();
        assert_eq!((width, height), (640, 32));
    }
}
