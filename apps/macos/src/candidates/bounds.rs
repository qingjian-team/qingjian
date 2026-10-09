//! 视图该有的大小，与其中内容区（窗口本体，不含投影留边）的位置。

use objc2_foundation::{NSRect, NSSize};

#[derive(Debug, Clone, Copy)]
pub struct ViewBounds {
    /// 整个视图（点），含主题投影留出的边。
    pub size: NSSize,

    /// 内容区（点），原点是视图左下角（AppKit 坐标），窗口按它贴光标。
    pub content: NSRect,
}

impl ViewBounds {
    /// 没有留边：内容区就是整个视图。
    pub fn filled(size: NSSize) -> Self {
        Self {
            size,
            content: NSRect::new(objc2_foundation::NSPoint::ZERO, size),
        }
    }
}
