//! 这次复制来自哪个程序、程序的图标，以及复制的文件总共多大。

use std::ptr;

use bamboo_slip::tiny_skia::{IntSize, Pixmap};
use objc2::AllocAnyThread;
use objc2_app_kit::{
    NSBitmapImageRep, NSCompositingOperation, NSDeviceRGBColorSpace, NSGraphicsContext, NSWorkspace,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

/// 图标画多大（像素）：卡片上 18 点，Retina 屏够用还有余。
const ICON_PIXELS: usize = 64;

/// 文件数超过这么多就不算大小，免得一次复制几万个文件卡住。
const MAX_SIZED_FILES: usize = 1000;

/// 前台程序的 .app 路径：macOS 不记剪贴板是谁写的，复制几乎都发生在前台程序里。不知道时为 `None`。
pub fn frontmost_app() -> Option<String> {
    let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
    let url = app.bundleURL().or_else(|| app.executableURL())?;
    Some(url.path()?.to_string())
}

/// 程序的图标；取不到为 `None`。只在主线程调（要画进位图）。
pub fn load_icon(path: &str) -> Option<Pixmap> {
    let icon = NSWorkspace::sharedWorkspace().iconForFile(&NSString::from_str(path));
    let side = ICON_PIXELS as isize;
    // SAFETY: 让 AppKit 自己分配像素缓冲；参数是 8 位 RGBA、无平面、按行紧排。
    let rep = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            ptr::null_mut(),
            side,
            side,
            8,
            4,
            true,
            false,
            NSDeviceRGBColorSpace,
            side * 4,
            32,
        )
    }?;
    let context = NSGraphicsContext::graphicsContextWithBitmapImageRep(&rep)?;
    NSGraphicsContext::saveGraphicsState_class();
    NSGraphicsContext::setCurrentContext(Some(&context));
    let full = NSRect::new(
        NSPoint::new(0.0, 0.0),
        NSSize::new(side as f64, side as f64),
    );
    icon.drawInRect_fromRect_operation_fraction(
        full,
        NSRect::ZERO,
        NSCompositingOperation::SourceOver,
        1.0,
    );
    context.flushGraphics();
    NSGraphicsContext::restoreGraphicsState_class();
    let pixels = rep.bitmapData();
    if pixels.is_null() {
        return None;
    }
    // SAFETY: 位图是上面按 64×64×4 字节、按行紧排建的，画完后整块可读；数据是预乘过透明度的 RGBA，与 tiny-skia 一致。
    let bytes = unsafe { std::slice::from_raw_parts(pixels, ICON_PIXELS * ICON_PIXELS * 4) };
    Pixmap::from_vec(
        bytes.to_vec(),
        IntSize::from_wh(ICON_PIXELS as u32, ICON_PIXELS as u32)?,
    )
}

/// 文件的总大小；有文件夹、文件太多或读不到时为 `None`。
pub fn total_size(paths: &[String]) -> Option<u64> {
    if paths.len() > MAX_SIZED_FILES {
        return None;
    }
    let mut total = 0_u64;
    for path in paths {
        let metadata = std::fs::metadata(path).ok()?;
        if !metadata.is_file() {
            return None;
        }
        total = total.saturating_add(metadata.len());
    }
    Some(total)
}
