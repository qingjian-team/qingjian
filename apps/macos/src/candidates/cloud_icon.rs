//! 联想条目前的小云朵：SF Symbols 的 `cloud`，按主题着色；系统太旧拿不到符号时退回文字 `☁︎`。

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
        let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(
            &NSString::from_str("cloud"),
            Some(&NSString::from_str("云联想")),
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
