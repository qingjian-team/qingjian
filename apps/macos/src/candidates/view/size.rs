//! AppKit 逐项绘制时候选窗要多大：顶部拼音行、竖排、横排各自的宽高。

use objc2::DefinedClass;
use objc2_foundation::NSSize;
use qingjian_platform::LayoutMode;

use super::{
    CARET_WIDTH, CLOUD_FALLBACK, CLOUD_GAP, CandidateView, HIGHLIGHT_INSET, INDEX_GAP, SENTENCE_GAP,
};
use crate::candidates::frame::Frame;

impl CandidateView {
    pub(super) fn preferred_size(&self) -> NSSize {
        let theme = self.theme();
        let frame = self.ivars().frame.borrow();
        let (top_width, top_height) = self.top_line_size(&frame);
        let (body_width, body_height) = match self.ivars().layout.get() {
            LayoutMode::Vertical => self.vertical_size(&frame),
            LayoutMode::Horizontal if frame.columns > 0 => self.matrix_size(&frame),
            LayoutMode::Horizontal => self.horizontal_size(&frame),
        };
        let width = top_width.max(body_width);
        NSSize::new(
            width + theme.padding * 2.0,
            top_height + body_height + theme.padding * 2.0,
        )
    }

    /// 顶部拼音行（含右侧整句补全）需要的宽高；没有这一行时都是 0。
    pub(super) fn top_line_size(&self, frame: &Frame) -> (f64, f64) {
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

    pub(super) fn vertical_size(&self, frame: &Frame) -> (f64, f64) {
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

    pub(super) fn horizontal_size(&self, frame: &Frame) -> (f64, f64) {
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
    pub(super) fn highlighted_annotation_size(&self, frame: &Frame) -> Option<(f64, f64)> {
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
    pub(super) fn cloud_width(&self) -> f64 {
        let cloud = &self.ivars().cloud;
        let width = if cloud.is_symbol() {
            cloud.width()
        } else {
            self.measure(CLOUD_FALLBACK, &self.theme().annotation_font)
                .width
        };
        width + CLOUD_GAP
    }
}
