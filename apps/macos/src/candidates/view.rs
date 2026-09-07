//! 候选窗口的内容视图：自绘顶部拼音行与若干候选，竖排一行一个、横排排成一行，一项高亮。

use std::cell::{Cell, RefCell};

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSAttributedStringNSStringDrawing, NSBezierPath, NSColor, NSFont, NSFontAttributeName,
    NSForegroundColorAttributeName, NSStrikethroughStyleAttributeName, NSView,
};
use objc2_foundation::{
    NSAttributedString, NSDictionary, NSNumber, NSPoint, NSRect, NSSize, NSString,
};
use qingjian_platform::LayoutMode;

use super::cloud_icon::CloudIcon;
use super::frame::Frame;
use super::preedit::Preedit;
use super::preedit::PreeditStyle;
use super::row::{Row, Tone};
use super::theme::Theme;

/// 视图状态。
pub struct Ivars {
    /// 当前显示的一帧。
    frame: RefCell<Frame>,

    /// 竖排 / 横排。
    layout: Cell<LayoutMode>,

    /// 云联想的小云朵。
    cloud: CloudIcon,

    /// 主题。
    theme: Theme,
}

/// preedit 光标的宽度。
const CARET_WIDTH: f64 = 1.5;

/// 云朵图标边长。
const CLOUD_SIZE: f64 = 13.0;

/// 云朵与后面文字的间距。
const CLOUD_GAP: f64 = 4.0;

/// 拿不到 SF Symbol 时的文字云朵。
const CLOUD_FALLBACK: &str = "☁︎";

/// preedit 与右侧整句补全之间的间距。
const SENTENCE_GAP: f64 = 16.0;

/// 横排时序号与候选词之间的间距。
const INDEX_GAP: f64 = 3.0;

/// 横排时高亮底色在候选两侧多出的宽度。
const HIGHLIGHT_INSET: f64 = 5.0;

/// `NSUnderlineStyleSingle`：删除线用单线。
const STRIKE_SINGLE: isize = 1;

/// 竖排的列宽与行高。
struct Columns {
    index_width: f64,
    text_width: f64,
    annotation_width: f64,
    row_height: f64,
}

/// 横排时每一项的尺寸。
struct Item {
    index_width: f64,
    text_width: f64,
}

define_class!(
    // SAFETY: NSView 允许子类化；没有实现 Drop。
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ivars]
    pub struct CandidateView;

    impl CandidateView {
        /// 用左上角为原点的坐标系，行从上往下画。
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            self.draw();
        }
    }
);

impl CandidateView {
    pub fn new(mtm: MainThreadMarker, theme: Theme) -> Retained<Self> {
        let cloud = CloudIcon::new(&theme.cloud_color, CLOUD_SIZE);
        let this = mtm.alloc::<Self>().set_ivars(Ivars {
            frame: RefCell::new(Frame::default()),
            layout: Cell::new(LayoutMode::default()),
            cloud,
            theme,
        });
        unsafe { msg_send![super(this), initWithFrame: NSRect::ZERO] }
    }

    pub fn theme(&self) -> &Theme {
        &self.ivars().theme
    }

    pub fn set_layout(&self, layout: LayoutMode) {
        self.ivars().layout.set(layout);
    }

    /// 更新内容并返回需要的窗口尺寸。
    pub fn set_frame(&self, frame: &Frame) -> NSSize {
        *self.ivars().frame.borrow_mut() = frame.clone();
        self.setNeedsDisplay(true);
        self.preferred_size()
    }

    fn preferred_size(&self) -> NSSize {
        let theme = self.theme();
        let frame = self.ivars().frame.borrow();
        let (top_width, top_height) = self.top_line_size(&frame);
        let (body_width, body_height) = match self.ivars().layout.get() {
            LayoutMode::Vertical => self.vertical_size(&frame),
            LayoutMode::Horizontal => self.horizontal_size(&frame),
        };
        let width = top_width.max(body_width);
        NSSize::new(
            width + theme.padding * 2.0,
            top_height + body_height + theme.padding * 2.0,
        )
    }

    /// 顶部拼音行（含右侧整句补全）需要的宽高；没有这一行时都是 0。
    fn top_line_size(&self, frame: &Frame) -> (f64, f64) {
        if !frame.has_top_line() {
            return (0.0, 0.0);
        }
        let theme = self.theme();
        let line_height = self.measure("x", &theme.annotation_font).height;
        let mut width = 0.0;
        if let Some(preedit) = &frame.preedit {
            width += self.measure(&preedit.text(), &theme.annotation_font).width + CARET_WIDTH;
        }
        if let Some((text, cloud)) = frame.trailing() {
            if frame.preedit.is_some() {
                width += SENTENCE_GAP;
            }
            if cloud {
                width += self.cloud_width();
            }
            width += self.measure(text, &theme.annotation_font).width;
        }
        (width, line_height + theme.row_padding * 2.0)
    }

    fn vertical_size(&self, frame: &Frame) -> (f64, f64) {
        let theme = self.theme();
        let columns = self.columns(&frame.rows);
        let mut width = columns.index_width + theme.column_gap + columns.text_width;
        if columns.annotation_width > 0.0 {
            width += theme.column_gap + columns.annotation_width;
        }
        let mut height = columns.row_height * frame.rows.len() as f64;
        if let Some(footer) = frame.footer.as_deref() {
            let footer_size = self.measure(footer, &theme.index_font);
            width = width.max(footer_size.width);
            height += footer_size.height + theme.row_padding;
        }
        (width, height)
    }

    fn horizontal_size(&self, frame: &Frame) -> (f64, f64) {
        let theme = self.theme();
        if frame.rows.is_empty() {
            return (0.0, 0.0);
        }
        let (items, row_height) = self.items(&frame.rows);
        let mut width: f64 = items
            .iter()
            .map(|item| item.index_width + INDEX_GAP + item.text_width)
            .sum::<f64>()
            + theme.column_gap * (items.len().saturating_sub(1)) as f64
            + HIGHLIGHT_INSET * 2.0;
        if let Some(footer) = frame.footer.as_deref() {
            width += theme.column_gap + self.measure(footer, &theme.index_font).width;
        }
        let mut height = row_height;
        if let Some((annotation_width, annotation_height)) = self.highlighted_annotation_size(frame)
        {
            width = width.max(annotation_width);
            height += annotation_height;
        }
        (width, height)
    }

    /// 横排时高亮候选的译文行尺寸；高亮候选没有译文时为 `None`。
    fn highlighted_annotation_size(&self, frame: &Frame) -> Option<(f64, f64)> {
        let theme = self.theme();
        let row = frame.rows.get(frame.highlighted)?;
        if row.annotation.is_empty() {
            return None;
        }
        let width: f64 = row
            .annotation
            .iter()
            .map(|(s, _)| self.measure(s, &theme.annotation_font).width)
            .sum();
        let height = self.measure("x", &theme.annotation_font).height + theme.row_padding;
        Some((width, height))
    }

    /// 云朵图标占的宽度（含后面的间距）。
    fn cloud_width(&self) -> f64 {
        let cloud = &self.ivars().cloud;
        let width = if cloud.is_symbol() {
            cloud.width()
        } else {
            self.measure(CLOUD_FALLBACK, &self.theme().annotation_font)
                .width
        };
        width + CLOUD_GAP
    }

    /// 画云朵，返回占用宽度（含间距）。`top` 是所在行文字的顶边，`line_height` 用来垂直居中。
    fn draw_cloud(&self, x: f64, top: f64, line_height: f64) -> f64 {
        let cloud = &self.ivars().cloud;
        if cloud.is_symbol() {
            cloud.draw(x, top + (line_height - cloud.width()) / 2.0);
        } else {
            let theme = self.theme();
            let size = self.measure(CLOUD_FALLBACK, &theme.annotation_font);
            self.draw_text(
                CLOUD_FALLBACK,
                &theme.annotation_font,
                &theme.cloud_color,
                top + (line_height - size.height) / 2.0,
                x,
            );
        }
        self.cloud_width()
    }

    fn columns(&self, rows: &[Row]) -> Columns {
        let theme = self.theme();
        let mut columns = Columns {
            index_width: 0.0,
            text_width: 0.0,
            annotation_width: 0.0,
            row_height: 0.0,
        };
        for row in rows {
            let index = self.measure(&row.index, &theme.index_font);
            let mut text = self.measure(&row.text, &theme.text_font);
            if row.cloud {
                text.width += self.cloud_width();
            }
            let annotation: f64 = row
                .annotation
                .iter()
                .map(|(s, _)| self.measure(s, &theme.annotation_font).width)
                .sum();
            columns.index_width = columns.index_width.max(index.width);
            columns.text_width = columns.text_width.max(text.width);
            columns.annotation_width = columns.annotation_width.max(annotation);
            columns.row_height = columns
                .row_height
                .max(text.height + theme.row_padding * 2.0);
        }
        columns
    }

    /// 横排各项的尺寸与统一行高。
    fn items(&self, rows: &[Row]) -> (Vec<Item>, f64) {
        let theme = self.theme();
        let mut row_height: f64 = 0.0;
        let items = rows
            .iter()
            .map(|row| {
                let index = self.measure(&row.index, &theme.index_font);
                let mut text = self.measure(&row.text, &theme.text_font);
                if row.cloud {
                    text.width += self.cloud_width();
                }
                row_height = row_height.max(text.height + theme.row_padding * 2.0);
                Item {
                    index_width: index.width,
                    text_width: text.width,
                }
            })
            .collect();
        (items, row_height)
    }

    fn draw(&self) {
        let theme = self.theme();
        let frame = self.ivars().frame.borrow();
        let bounds = self.bounds();

        // 背景
        theme.background.set();
        NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(
            bounds,
            theme.corner_radius,
            theme.corner_radius,
        )
        .fill();

        let mut y = theme.padding;
        y += self.draw_top_line(&frame, y);
        match self.ivars().layout.get() {
            LayoutMode::Vertical => self.draw_vertical(&frame, y, bounds),
            LayoutMode::Horizontal => self.draw_horizontal(&frame, y, bounds),
        }
    }

    /// 顶部拼音行：各段按样式画、我们自己画光标（不依赖应用画插入点）、右侧整句补全。返回占用高度。
    fn draw_top_line(&self, frame: &Frame, y: f64) -> f64 {
        if !frame.has_top_line() {
            return 0.0;
        }
        let theme = self.theme();
        let line_height = self.measure("x", &theme.annotation_font).height;
        let top = y + theme.row_padding;
        let mut x = theme.padding;
        if let Some(preedit) = &frame.preedit {
            x += self.draw_preedit(preedit, x, top, line_height);
            if frame.trailing().is_some() {
                x += SENTENCE_GAP;
            }
        }
        // 整句补全：云朵 + 句子，颜色与本地候选区分；临时状态灰字、不带云朵
        if let Some((text, cloud)) = frame.trailing() {
            let color = if cloud {
                x += self.draw_cloud(x, top, line_height);
                &theme.cloud_color
            } else {
                &theme.gloss_color
            };
            self.draw_text(text, &theme.annotation_font, color, top, x);
        }
        line_height + theme.row_padding * 2.0
    }

    /// 画拼音行的各段与光标，返回占用宽度（含光标）。
    fn draw_preedit(&self, preedit: &Preedit, x: f64, top: f64, line_height: f64) -> f64 {
        let theme = self.theme();
        let mut cursor_x = x;
        for segment in &preedit.segments {
            let (color, strike) = match segment.style {
                PreeditStyle::Typed => (&theme.gloss_color, false),
                PreeditStyle::Rest => (&theme.pos_color, false),
                PreeditStyle::Struck => (&theme.pos_color, true),
            };
            let string = self.attributed(&segment.text, &theme.annotation_font, color, strike);
            string.drawAtPoint(NSPoint::new(cursor_x, top));
            cursor_x += string.size().width;
        }
        let caret_x = x + self
            .measure(&preedit.before_cursor(), &theme.annotation_font)
            .width;
        theme.text_color.set();
        NSBezierPath::fillRect(NSRect::new(
            NSPoint::new(caret_x, top),
            NSSize::new(CARET_WIDTH, line_height),
        ));
        cursor_x - x + CARET_WIDTH
    }

    fn draw_vertical(&self, frame: &Frame, mut y: f64, bounds: NSRect) {
        let theme = self.theme();
        let columns = self.columns(&frame.rows);
        let text_x = theme.padding + columns.index_width + theme.column_gap;
        let annotation_x = text_x + columns.text_width + theme.column_gap;
        for (i, row) in frame.rows.iter().enumerate() {
            if i == frame.highlighted {
                let rect = NSRect::new(
                    NSPoint::new(theme.padding / 2.0, y),
                    NSSize::new(bounds.size.width - theme.padding, columns.row_height),
                );
                self.fill_highlight(rect);
            }
            // 各列底部对齐到候选词基线附近：小字往下挪一点
            let text_size = self.measure(&row.text, &theme.text_font);
            let baseline = y + theme.row_padding;
            let small_offset = self.small_offset(text_size.height);
            self.draw_text(
                &row.index,
                &theme.index_font,
                &theme.index_color,
                baseline + small_offset,
                theme.padding,
            );
            self.draw_word(row, text_x, baseline, text_size.height);
            let mut x = annotation_x;
            for (segment, tone) in &row.annotation {
                x += self.draw_text(
                    segment,
                    &theme.annotation_font,
                    self.tone_color(*tone),
                    baseline + small_offset,
                    x,
                );
            }
            y += columns.row_height;
        }
        if let Some(footer) = frame.footer.as_deref() {
            let size = self.measure(footer, &theme.index_font);
            self.draw_text(
                footer,
                &theme.index_font,
                &theme.index_color,
                y + theme.row_padding,
                bounds.size.width - theme.padding - size.width,
            );
        }
    }

    /// 横排：候选排成一行，高亮那个下面单独一行译文，页码在行尾。
    fn draw_horizontal(&self, frame: &Frame, y: f64, bounds: NSRect) {
        if frame.rows.is_empty() {
            return;
        }
        let theme = self.theme();
        let (items, row_height) = self.items(&frame.rows);
        let baseline = y + theme.row_padding;
        let mut x = theme.padding + HIGHLIGHT_INSET;
        for (i, (row, item)) in frame.rows.iter().zip(&items).enumerate() {
            let item_width = item.index_width + INDEX_GAP + item.text_width;
            if i == frame.highlighted {
                let rect = NSRect::new(
                    NSPoint::new(x - HIGHLIGHT_INSET, y),
                    NSSize::new(item_width + HIGHLIGHT_INSET * 2.0, row_height),
                );
                self.fill_highlight(rect);
            }
            let text_size = self.measure(&row.text, &theme.text_font);
            self.draw_text(
                &row.index,
                &theme.index_font,
                &theme.index_color,
                baseline + self.small_offset(text_size.height),
                x,
            );
            self.draw_word(
                row,
                x + item.index_width + INDEX_GAP,
                baseline,
                text_size.height,
            );
            x += item_width + theme.column_gap;
        }
        if let Some(footer) = frame.footer.as_deref() {
            let size = self.measure(footer, &theme.index_font);
            let text_height = self.measure("x", &theme.text_font).height;
            self.draw_text(
                footer,
                &theme.index_font,
                &theme.index_color,
                baseline + self.small_offset(text_height),
                bounds.size.width - theme.padding - size.width,
            );
        }
        // 高亮候选的译文
        if let Some(row) = frame.rows.get(frame.highlighted) {
            let mut x = theme.padding + HIGHLIGHT_INSET;
            let top = y + row_height + theme.row_padding / 2.0;
            for (segment, tone) in &row.annotation {
                x += self.draw_text(
                    segment,
                    &theme.annotation_font,
                    self.tone_color(*tone),
                    top,
                    x,
                );
            }
        }
    }

    /// 候选词本体：云端词前带云朵、换颜色。
    fn draw_word(&self, row: &Row, x: f64, baseline: f64, text_height: f64) {
        let theme = self.theme();
        let mut word_x = x;
        if row.cloud {
            word_x += self.draw_cloud(word_x, baseline, text_height);
        }
        let color = if row.cloud {
            &theme.cloud_color
        } else {
            &theme.text_color
        };
        self.draw_text(&row.text, &theme.text_font, color, baseline, word_x);
    }

    fn fill_highlight(&self, rect: NSRect) {
        let theme = self.theme();
        theme.highlight.set();
        NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(
            rect,
            theme.corner_radius / 2.0,
            theme.corner_radius / 2.0,
        )
        .fill();
    }

    /// 小字相对候选词往下挪多少，让两者底部对齐。
    fn small_offset(&self, text_height: f64) -> f64 {
        (text_height - self.measure("x", &self.theme().annotation_font).height).max(0.0)
    }

    fn tone_color(&self, tone: Tone) -> &NSColor {
        match tone {
            Tone::Gloss => &self.theme().gloss_color,
            Tone::Fresh => &self.theme().fresh_color,
            Tone::Faint => &self.theme().pos_color,
        }
    }

    /// 画一段文字，返回它的宽度。参数顺序是 (顶边 y, 左边 x)，与画图时「先定行再定列」的习惯一致。
    fn draw_text(&self, text: &str, font: &NSFont, color: &NSColor, y: f64, x: f64) -> f64 {
        let string = self.attributed(text, font, color, false);
        string.drawAtPoint(NSPoint::new(x, y));
        string.size().width
    }

    fn measure(&self, text: &str, font: &NSFont) -> NSSize {
        self.attributed(text, font, &self.theme().text_color, false)
            .size()
    }

    fn attributed(
        &self,
        text: &str,
        font: &NSFont,
        color: &NSColor,
        strike: bool,
    ) -> Retained<NSAttributedString> {
        let strike_style = NSNumber::new_isize(STRIKE_SINGLE);
        // SAFETY: 只读 AppKit 导出的属性名常量
        let (keys, objects): (Vec<&NSString>, Vec<&AnyObject>) = unsafe {
            if strike {
                (
                    vec![
                        NSFontAttributeName,
                        NSForegroundColorAttributeName,
                        NSStrikethroughStyleAttributeName,
                    ],
                    vec![font, color, &strike_style],
                )
            } else {
                (
                    vec![NSFontAttributeName, NSForegroundColorAttributeName],
                    vec![font, color],
                )
            }
        };
        let attributes = NSDictionary::from_slices(&keys, &objects);
        unsafe { NSAttributedString::new_with_attributes(&NSString::from_str(text), &attributes) }
    }
}
