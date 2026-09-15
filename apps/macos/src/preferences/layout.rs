//! 页内布局器：自上而下摆控件，并把每一组控件装进一张圆角卡片（系统设置那种分组样式）。

use objc2::Message;
use objc2::rc::Retained;
use objc2_app_kit::{NSBox, NSBoxType, NSColor, NSTitlePosition, NSView};
use objc2_foundation::{NSPoint, NSRect, NSSize};

/// 行高。
pub const ROW_HEIGHT: f64 = 24.0;

/// 行距。
pub const ROW_GAP: f64 = 8.0;

/// 卡片离页面左右边缘的距离。
pub const CARD_MARGIN: f64 = 20.0;

/// 卡片内左右留白。
const CARD_PAD_X: f64 = 14.0;

/// 卡片内上下留白。
const CARD_PAD_Y: f64 = 12.0;

/// 卡片之间的距离。
const CARD_GAP: f64 = 14.0;

/// 卡片圆角。
const CARD_RADIUS: f64 = 8.0;

/// 页内左右留白：控件从卡片内侧起摆。
pub const PAGE_PADDING: f64 = CARD_MARGIN + CARD_PAD_X;

/// 标题列的宽度（标题右对齐贴着控件）。
pub const LABEL_WIDTH: f64 = 110.0;

/// 控件列的 x：标题列 + 间隔。
pub const CONTROL_X: f64 = PAGE_PADDING + LABEL_WIDTH + 10.0;

/// 每页的内容宽度。
pub const PAGE_WIDTH: f64 = 560.0;

/// 在一页里自上而下摆控件的简易布局：AppKit 坐标原点在左下，先按「离顶部多远」记下来，
/// 最后知道页高了再一次性换算成 frame。
///
/// 控件按「组」归到卡片里：[`Layout::end_group`] 结束当前组并空出组间距，`finish` 时每组后面垫一张卡片。
/// `plain` 建的布局器不画卡片（自定义短语的编辑表单用）。
pub struct Layout {
    /// 这一页的宽度。
    width: f64,

    /// 已摆的控件及其（x, 顶部距离, 宽, 高）。
    placed: Vec<(Retained<NSView>, f64, f64, f64, f64)>,

    /// 当前行的顶部距离。
    top: f64,

    /// 要不要画卡片。
    cards: bool,

    /// 当前组第一行的顶部距离。
    group_start: f64,

    /// 当前组里最靠下的控件底部；没摆控件时等于 `group_start`。
    group_bottom: f64,

    /// 已经结束的组：卡片的（顶部距离, 底部距离）。
    groups: Vec<(f64, f64)>,
}

impl Layout {
    /// 带卡片的布局器；`top_margin` 是第一张卡片离页顶的距离。
    pub fn new(width: f64, top_margin: f64) -> Self {
        let start = top_margin + CARD_PAD_Y;
        Self {
            width,
            placed: Vec::new(),
            top: start,
            cards: true,
            group_start: start,
            group_bottom: start,
            groups: Vec::new(),
        }
    }

    /// 不画卡片的布局器：控件直接摆在 `top_margin` 以下。
    pub fn plain(width: f64, top_margin: f64) -> Self {
        Self {
            cards: false,
            ..Self::new(width, top_margin - CARD_PAD_Y)
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
        self.group_bottom = self.group_bottom.max(self.top + height);
        self.placed
            .push((view.retain(), x, self.top, width, height));
    }

    /// 换到下一行。
    pub fn next_row(&mut self, height: f64) {
        self.top += height + ROW_GAP;
    }

    /// 结束当前组：这一组的控件装进一张卡片，下一组从卡片下方隔 [`CARD_GAP`] 开始。空组不出卡片。
    pub fn end_group(&mut self) {
        if self.group_bottom <= self.group_start {
            return;
        }
        let card_bottom = self.group_bottom + CARD_PAD_Y;
        self.groups
            .push((self.group_start - CARD_PAD_Y, card_bottom));
        self.top = card_bottom + CARD_GAP + CARD_PAD_Y;
        self.group_start = self.top;
        self.group_bottom = self.top;
    }

    /// 到目前为止用掉的高度（含顶部留白与最后一张卡片的底边；不画卡片时就是当前行的顶部）。
    pub fn height(&self) -> f64 {
        if !self.cards {
            self.top
        } else if self.group_bottom > self.group_start {
            self.group_bottom + CARD_PAD_Y
        } else if let Some((_, bottom)) = self.groups.last() {
            *bottom
        } else {
            self.top
        }
    }

    /// 把所有控件加进容器并按容器高度设好 frame（顶部对齐）；卡片先加，垫在控件下面。
    pub fn finish(mut self, container: &NSView, total_height: f64) {
        self.end_group();
        if self.cards {
            for (top, bottom) in &self.groups {
                let card = card(container);
                card.setFrame(NSRect::new(
                    NSPoint::new(CARD_MARGIN, total_height - bottom),
                    NSSize::new(self.width - 2.0 * CARD_MARGIN, bottom - top),
                ));
                container.addSubview(&card);
            }
        }
        for (view, x, top, width, height) in self.placed {
            let y = total_height - top - height;
            view.setFrame(NSRect::new(NSPoint::new(x, y), NSSize::new(width, height)));
            container.addSubview(&view);
        }
    }
}

/// 一张卡片：圆角、控件背景色、细分隔线描边，深浅色跟系统。
fn card(container: &NSView) -> Retained<NSBox> {
    let mtm = objc2::MainThreadMarker::from(container);
    let card = NSBox::initWithFrame(mtm.alloc(), NSRect::ZERO);
    card.setBoxType(NSBoxType::Custom);
    card.setTitlePosition(NSTitlePosition::NoTitle);
    card.setContentViewMargins(NSSize::ZERO);
    card.setCornerRadius(CARD_RADIUS);
    card.setBorderWidth(1.0);
    card.setBorderColor(&NSColor::separatorColor());
    card.setFillColor(&NSColor::controlBackgroundColor());
    card
}
