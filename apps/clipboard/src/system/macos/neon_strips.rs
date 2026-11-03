//! [`Strips`]：所有显示器四条边上的透明窗口，以及什么时候不流动。
//! 像素在霓虹灯线程上算（[`Edge`]），贴到窗口上交给主线程；窗口本身只放在主线程的 `WINDOWS` 里。

use std::cell::RefCell;
use std::ptr;

use objc2::rc::Retained;
use objc2::{AllocAnyThread, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSBackingStoreType, NSBitmapImageRep, NSColor, NSDeviceRGBColorSpace, NSImage, NSImageView,
    NSScreen, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask, NSWorkspace,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};

use super::main_queue::run_on_main;
use crate::system::neon::{Edge, PixelOrder};

/// 灯条厚度的上下限（像素），与 Windows 一致。
const THICKNESS: (i32, i32) = (20, 64);

/// `NSStatusWindowLevel`：盖在菜单栏上面，灯条才绕得满一圈。
const STATUS_WINDOW_LEVEL: isize = 25;

thread_local! {
    /// 灯条窗口与贴图的视图，和 [`Strips`] 里的边一一对应。只在主线程碰。
    static WINDOWS: RefCell<Vec<(Retained<NSWindow>, Retained<NSImageView>)>> =
        const { RefCell::new(Vec::new()) };
}

/// 一条灯条：像素网格上的形状，以及显示多大（点）。
struct Strip {
    edge: Edge,

    points: NSSize,
}

pub(crate) struct Strips(Vec<Strip>);

fn open_window(
    mtm: MainThreadMarker,
    frame: NSRect,
) -> (Retained<NSWindow>, Retained<NSImageView>) {
    // SAFETY: 无边框窗口的标准建法；不让它关闭时自己释放，留给 Retained 管。
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            frame,
            NSWindowStyleMask::Borderless,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    // SAFETY: 见上。
    unsafe { window.setReleasedWhenClosed(false) };
    window.setOpaque(false);
    window.setBackgroundColor(Some(&NSColor::clearColor()));
    window.setHasShadow(false);
    window.setIgnoresMouseEvents(true);
    window.setLevel(STATUS_WINDOW_LEVEL);
    window.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::IgnoresCycle,
    );
    let view = NSImageView::new(mtm);
    window.setContentView(Some(&view));
    window.orderFrontRegardless();
    (window, view)
}

/// 一帧像素包成图：位图按像素，图按点，Retina 上一个点几个像素。
fn image(pixels: &[u8], edge: &Edge, points: NSSize) -> Option<Retained<NSImage>> {
    let (width, height) = edge.size;
    // SAFETY: 让 AppKit 自己分配像素缓冲；8 位预乘 RGBA、无平面、按行紧排。
    let rep = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            ptr::null_mut(),
            width as isize,
            height as isize,
            8,
            4,
            true,
            false,
            NSDeviceRGBColorSpace,
            width as isize * 4,
            32,
        )
    }?;
    let target = rep.bitmapData();
    if target.is_null() {
        return None;
    }
    // SAFETY: 位图按宽 × 高 × 4 字节建，pixels 由同一个 Edge 画出来，大小相同。
    unsafe { ptr::copy_nonoverlapping(pixels.as_ptr(), target, pixels.len()) };
    let image = NSImage::initWithSize(NSImage::alloc(), points);
    image.addRepresentation(&rep);
    Some(image)
}

impl Strips {
    pub(crate) fn create() -> Self {
        let strips = run_on_main(|mtm| {
            let screens = NSScreen::screens(mtm);
            let mut strips = Vec::new();
            let mut windows = Vec::new();
            for screen in &screens {
                let frame = screen.frame();
                let scale = screen.backingScaleFactor();
                let pixels = |points: f64| (points * scale).round() as i32;
                let monitor = (pixels(frame.size.width), pixels(frame.size.height));
                for edge in Edge::around(monitor, THICKNESS) {
                    let (x, y) = edge.offset();
                    let size = NSSize::new(
                        f64::from(edge.size.0) / scale,
                        f64::from(edge.size.1) / scale,
                    );
                    // 灯条的偏移是左上角起、y 朝下；AppKit 的屏幕坐标原点在左下、y 朝上
                    let origin = NSPoint::new(
                        frame.origin.x + f64::from(x) / scale,
                        frame.origin.y + frame.size.height - f64::from(y) / scale - size.height,
                    );
                    windows.push(open_window(mtm, NSRect::new(origin, size)));
                    strips.push(Strip { edge, points: size });
                }
            }
            WINDOWS.set(windows);
            strips
        });
        Self(strips)
    }

    pub(crate) fn draw(&self, color: [f32; 3], time: f32, strength: f32) {
        let frames: Vec<Vec<u8>> = self
            .0
            .iter()
            .map(|strip| {
                let (width, height) = strip.edge.size;
                let mut pixels = vec![0_u8; (width * height * 4) as usize];
                strip
                    .edge
                    .paint(&mut pixels, PixelOrder::Rgba, color, time, strength);
                pixels
            })
            .collect();
        let shapes: Vec<(Edge, NSSize)> = self
            .0
            .iter()
            .map(|strip| (strip.edge, strip.points))
            .collect();
        run_on_main(move |_| {
            WINDOWS.with_borrow(|windows| {
                for ((_, view), (pixels, (edge, points))) in
                    windows.iter().zip(frames.iter().zip(&shapes))
                {
                    if let Some(image) = image(pixels, edge, *points) {
                        view.setImage(Some(&image));
                    }
                }
            });
        });
    }
}

impl Drop for Strips {
    fn drop(&mut self) {
        run_on_main(|_| {
            for (window, _) in WINDOWS.take() {
                window.orderOut(None);
                window.close();
            }
        });
    }
}

/// macOS 没有 Windows 那样「全屏游戏 / 演示中」的查询，一直照常亮。
pub(crate) fn should_hide() -> bool {
    false
}

/// 系统「减少动态效果」开着就不流动，只亮和变色。
pub(crate) fn animations_on() -> bool {
    !NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion()
}
