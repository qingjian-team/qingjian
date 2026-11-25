//! macOS 带毛玻璃的窗口怎么贴图：窗口内容视图后面垫一张 `NSVisualEffectView`（侧栏材质、透出窗口背后），
//! 内容视图的层上挂一张自己的 CALayer，每帧贴一张预乘 alpha 的 CGImage；位图里半透明的地方就透出毛玻璃。

use std::ptr;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
    NSVisualEffectState, NSVisualEffectView, NSWindowOrderingMode,
};
use objc2_core_foundation::{CFData, CFRetained, CGPoint};
use objc2_core_graphics::{
    CGBitmapInfo, CGColorRenderingIntent, CGColorSpace, CGDataProvider, CGImage, CGImageAlphaInfo,
    CGImageByteOrderInfo, CGImageComponentInfo, CGImagePixelFormatInfo,
};
use objc2_quartz_core::{CALayer, CATransaction, kCAGravityTopLeft};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

pub(in crate::host) struct Layer {
    /// 内容视图的层，自己那张层跟着它的大小。
    root: Retained<CALayer>,

    layer: Retained<CALayer>,

    space: CFRetained<CGColorSpace>,

    /// 留着毛玻璃视图的引用，窗口在它就在。
    _effect: Retained<NSVisualEffectView>,
}

impl Layer {
    /// 拿不到内容视图（不是 AppKit 窗口、不在主线程）时返回 `None`。
    pub(super) fn new(window: &Window) -> Option<Self> {
        let mtm = MainThreadMarker::new()?;
        let handle = window.window_handle().ok()?;
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            return None;
        };
        // SAFETY: winit 给的 `ns_view` 是这个窗口还活着的内容视图
        let view: Retained<NSView> = unsafe { Retained::retain(handle.ns_view.as_ptr().cast()) }?;
        // SAFETY: 在主线程上读视图树
        let parent = unsafe { view.superview() }?;

        let effect = NSVisualEffectView::initWithFrame(mtm.alloc(), view.frame());
        effect.setMaterial(NSVisualEffectMaterial::Sidebar);
        effect.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
        effect.setState(NSVisualEffectState::FollowsWindowActiveState);
        effect.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        parent.addSubview_positioned_relativeTo(&effect, NSWindowOrderingMode::Below, Some(&view));

        view.setWantsLayer(true);
        let root = view.layer()?;
        let layer = CALayer::new();
        layer.setAnchorPoint(CGPoint::new(0.0, 0.0));
        layer.setGeometryFlipped(true);
        // SAFETY: 系统给的常量
        layer.setContentsGravity(unsafe { kCAGravityTopLeft });
        root.addSublayer(&layer);
        Some(Self {
            root,
            layer,
            space: CGColorSpace::new_device_rgb()?,
            _effect: effect,
        })
    }

    /// `pixels` 是 `width × height` 个预乘的 `0xAARRGGBB`；`scale` 是窗口的像素倍数。
    pub(super) fn present(&self, pixels: &[u32], width: u32, height: u32, scale: f64) {
        let (width, height) = (width as usize, height as usize);
        let bytes = pixels.len() * 4;
        // SAFETY: 指针与长度来自同一个切片；CFData 把字节拷走
        let Some(data) = (unsafe { CFData::new(None, pixels.as_ptr().cast(), bytes as isize) })
        else {
            return;
        };
        let Some(provider) = CGDataProvider::with_cf_data(Some(&data)) else {
            return;
        };
        let info = CGBitmapInfo(
            CGImageAlphaInfo::PremultipliedFirst.0
                | CGImageComponentInfo::Integer.0
                | CGImageByteOrderInfo::Order32Little.0
                | CGImagePixelFormatInfo::Packed.0,
        );
        // SAFETY: `decode` 传空；尺寸、行宽与数据长度一致
        let image = unsafe {
            CGImage::new(
                width,
                height,
                8,
                32,
                width * 4,
                Some(&self.space),
                info,
                Some(&provider),
                ptr::null(),
                false,
                CGColorRenderingIntent::RenderingIntentDefault,
            )
        };
        let Some(image) = image else {
            tracing::warn!(width, height, "CGImage creation failed");
            return;
        };
        // 换内容默认带四分之一秒的淡入，关掉
        CATransaction::begin();
        CATransaction::setDisableActions(true);
        self.layer.setFrame(self.root.bounds());
        self.layer.setContentsScale(scale);
        // SAFETY: 层的内容可以是 CGImage
        unsafe { self.layer.setContents(Some(image.as_ref())) };
        CATransaction::commit();
    }
}
