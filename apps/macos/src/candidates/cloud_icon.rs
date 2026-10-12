//! SF Symbols 小图标：云联想用 `cloud`，常用语用 `cube`；系统太旧拿不到符号时退回文字。

use objc2::rc::Retained;
use objc2_app_kit::{NSColor, NSCompositingOperation, NSImage, NSImageSymbolConfiguration};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

pub struct CloudIcon {
    /// 已着色的符号图片；`None` 表示用文字兜底。
    image: Option<Retained<NSImage>>,

    /// 绘制尺寸（正方形边长）。
    size: f64,
}

impl CloudIcon {
    pub fn new(color: &NSColor, size: f64) -> Self {
        Self::symbol("cloud", "云联想", color, size)
    }

    /// 常用语右上角的小立方体。
    pub fn phrase(color: &NSColor, size: f64) -> Self {
        Self::symbol("cube", "常用语", color, size)
    }

    fn symbol(name: &str, label: &str, color: &NSColor, size: f64) -> Self {
        let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(
            &NSString::from_str(name),
            Some(&NSString::from_str(label)),
        )
        .and_then(|image| {
            let config = NSImageSymbolConfiguration::configurationWithHierarchicalColor(color);
            image.imageWithSymbolConfiguration(&config)
        });
        Self { image, size }
    }

    pub fn is_symbol(&self) -> bool {
        self.image.is_some()
    }

    /// 图标占的宽度（不含后面的间距）。
    pub fn width(&self) -> f64 {
        self.size
    }

    /// 在翻转坐标系里画到 `(x, y)`，`y` 是图标顶边。
    pub fn draw(&self, x: f64, y: f64) {
        let Some(image) = &self.image else {
            return;
        };
        let rect = NSRect::new(NSPoint::new(x, y), NSSize::new(self.size, self.size));
        // SAFETY: hints 传 None，其余参数都是普通值；在 drawRect: 内调用，有当前图形上下文。
        unsafe {
            image.drawInRect_fromRect_operation_fraction_respectFlipped_hints(
                rect,
                NSRect::ZERO,
                NSCompositingOperation::SourceOver,
                1.0,
                true,
                None,
            );
        }
    }
}
