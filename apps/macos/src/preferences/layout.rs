use objc2::Message;
use objc2::rc::Retained;
use objc2_app_kit::NSView;
use objc2_foundation::{NSPoint, NSRect, NSSize};

/// 行高。
pub const ROW_HEIGHT: f64 = 24.0;

/// 行距。
pub const ROW_GAP: f64 = 8.0;

/// 页内左右留白。
pub const PAGE_PADDING: f64 = 20.0;

/// 标题列的宽度（标题右对齐贴着控件）。
pub const LABEL_WIDTH: f64 = 110.0;

/// 控件列的 x：标题列 + 间隔。
pub const CONTROL_X: f64 = PAGE_PADDING + LABEL_WIDTH + 10.0;

/// 每个标签页的内容宽度。
pub const PAGE_WIDTH: f64 = 520.0;

/// 在一页里自上而下摆控件的简易布局：AppKit 坐标原点在左下，先按「离顶部多远」记下来，
/// 最后知道页高了再一次性换算成 frame。
pub struct Layout {
    /// 这一页的宽度。
    width: f64,

    /// 已摆的控件及其（x, 顶部距离, 宽, 高）。
    placed: Vec<(Retained<NSView>, f64, f64, f64, f64)>,

    /// 当前行的顶部距离。
    top: f64,
}

impl Layout {
    pub fn new(width: f64, top_margin: f64) -> Self {
        Self {
            width,
            placed: Vec::new(),
            top: top_margin,
        }
    }

    /// 控件列从 [`CONTROL_X`] 到右边留白之间的宽度。
    pub fn control_width(&self) -> f64 {
        self.width - CONTROL_X - PAGE_PADDING
    }

    /// 从左留白到右留白的整行宽度。
    pub fn inner_width(&self) -> f64 {
        self.width - 2.0 * PAGE_PADDING
    }

    /// 在当前行放一个控件。
    pub fn place(&mut self, view: &NSView, x: f64, width: f64, height: f64) {
        self.placed
            .push((view.retain(), x, self.top, width, height));
    }

    /// 换到下一行。
    pub fn next_row(&mut self, height: f64) {
        self.top += height + ROW_GAP;
    }

    /// 额外留白（分组之间）。
    pub fn space(&mut self, height: f64) {
        self.top += height;
    }

    /// 到目前为止用掉的高度（含顶部留白）。
    pub fn height(&self) -> f64 {
        self.top
    }

    /// 把所有控件加进容器并按容器高度设好 frame（顶部对齐）。
    pub fn finish(self, container: &NSView, total_height: f64) {
        for (view, x, top, width, height) in self.placed {
            let y = total_height - top - height;
            view.setFrame(NSRect::new(NSPoint::new(x, y), NSSize::new(width, height)));
            container.addSubview(&view);
        }
    }
}
