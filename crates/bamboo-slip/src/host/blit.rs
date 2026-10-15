//! 把画好的位图拷进 softbuffer 的缓冲（每像素 `0x00RRGGBB`）。

use tiny_skia::Pixmap;

/// 位图里内容区左上角 `(left, top)` 对齐窗口左上角；窗口比位图大的部分填黑。
/// 根节点铺不透明底色的应用预乘与否结果一样，直接取 RGB。
pub(super) fn blit(
    pixmap: &Pixmap,
    (left, top): (u32, u32),
    buffer: &mut [u32],
    width: u32,
    height: u32,
) {
    let pixels = pixmap.pixels();
    let stride = pixmap.width();
    for y in 0..height {
        let row = &mut buffer[(y * width) as usize..((y + 1) * width) as usize];
        let source_y = y + top;
        if source_y >= pixmap.height() {
            row.fill(0);
            continue;
        }
        let start = (source_y * stride) as usize;
        for (x, out) in row.iter_mut().enumerate() {
            let source_x = x as u32 + left;
            *out = if source_x < stride {
                let pixel = pixels[start + source_x as usize];
                (u32::from(pixel.red()) << 16)
                    | (u32::from(pixel.green()) << 8)
                    | u32::from(pixel.blue())
            } else {
                0
            };
        }
    }
}
