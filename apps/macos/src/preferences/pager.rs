//! 右侧内容列的翻页器：同一时间只显示一页，页顶一行大标题，窗口高度随所选页伸缩。

use std::cell::Cell;

use objc2::rc::Retained;
use objc2_app_kit::{NSFont, NSTextField, NSView, NSWindow};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

/// 内容列顶部的标题带高度（含透明标题栏），页从它下面开始。
pub const HEADER_HEIGHT: f64 = 52.0;

/// 页标题离标题带顶部的距离与字高。
const TITLE_TOP: f64 = 16.0;
const TITLE_HEIGHT: f64 = 24.0;

/// 一页：视图与它需要的高度。
pub struct PagerPage {
    /// 页名，显示在标题带里。
    pub title: &'static str,

    /// 承载视图，尺寸已定。
    pub view: Retained<NSView>,

    /// 视图高度。
    pub height: f64,
}

/// 翻页器：切页时改标题、只留一页可见，并把窗口高度改到这一页需要的高度。
pub struct Pager {
    /// 所属窗口，切页时改它的 frame。
    window: Retained<NSWindow>,

    /// 标题带里的页名。
    header: Retained<NSTextField>,

    /// 各页。
    pages: Vec<PagerPage>,

    /// 窗口内容视图的宽度。
    width: f64,

    /// 内容列在页之上与页之下固定占用的高度（标题带 + 状态行区域）。
    fixed_height: f64,

    /// 窗口内容的最低高度（侧栏要放得下）。
    min_height: f64,

    /// 当前显示的页号。
    current: Cell<Option<usize>>,
}

impl Pager {
    /// `page_x` 是内容列的左边；页视图与标题都放在这个 x 之后。
    pub fn new(
        window: Retained<NSWindow>,
        content: &NSView,
        pages: Vec<PagerPage>,
        page_x: f64,
        title_x: f64,
        fixed_height: f64,
        min_height: f64,
    ) -> Self {
        let width = content.frame().size.width;
        let header = NSTextField::labelWithString(
            &NSString::from_str(""),
            objc2::MainThreadMarker::from(content),
        );
        // SAFETY: NSFontWeightSemibold 是 AppKit 导出的常量，只读
        let weight = unsafe { objc2_app_kit::NSFontWeightSemibold };
        header.setFont(Some(&NSFont::systemFontOfSize_weight(17.0, weight)));
        header.setAutoresizingMask(objc2_app_kit::NSAutoresizingMaskOptions::ViewMinYMargin);
        content.addSubview(&header);
        for page in &pages {
            page.view
                .setAutoresizingMask(objc2_app_kit::NSAutoresizingMaskOptions::ViewMinYMargin);
            page.view.setHidden(true);
            content.addSubview(&page.view);
        }
        let pager = Self {
            window,
            header,
            pages,
            width,
            fixed_height,
            min_height,
            current: Cell::new(None),
        };
        pager.place(page_x, title_x, content.frame().size.height);
        pager
    }

    /// 按内容高度摆标题与各页：都贴着顶部，之后靠 autoresizing 跟着窗口顶走。
    fn place(&self, page_x: f64, title_x: f64, content_height: f64) {
        self.header.setFrame(NSRect::new(
            NSPoint::new(title_x, content_height - TITLE_TOP - TITLE_HEIGHT),
            NSSize::new(self.width - title_x, TITLE_HEIGHT),
        ));
        for page in &self.pages {
            page.view.setFrame(NSRect::new(
                NSPoint::new(page_x, content_height - HEADER_HEIGHT - page.height),
                NSSize::new(page.view.frame().size.width, page.height),
            ));
        }
    }

    /// 切到第 `index` 页。窗口已显示时高度变化带动画，否则直接设。
    pub fn show(&self, index: usize) {
        let Some(page) = self.pages.get(index) else {
            return;
        };
        if self.current.get() == Some(index) {
            return;
        }
        if let Some(previous) = self.current.get().and_then(|i| self.pages.get(i)) {
            previous.view.setHidden(true);
        }
        self.current.set(Some(index));
        self.header.setStringValue(&NSString::from_str(page.title));
        let height = (self.fixed_height + page.height).max(self.min_height);
        // 保持左上角不动：新 frame 的顶边对齐旧 frame 的顶边
        let content = NSRect::new(NSPoint::ZERO, NSSize::new(self.width, height));
        let mut frame = self.window.frameRectForContentRect(content);
        let old = self.window.frame();
        frame.origin = NSPoint::new(
            old.origin.x,
            old.origin.y + old.size.height - frame.size.height,
        );
        self.window
            .setFrame_display_animate(frame, true, self.window.isVisible());
        page.view.setHidden(false);
    }
}
