//! 横排：候选排成一行，高亮那个下面单独一行译文，页码在行尾。

use super::item::Item;
use super::{HIGHLIGHT_INSET, INDEX_GAP, Metrics, Renderer};
use crate::canvas::Canvas;
use crate::frame::{Frame, Row};

impl Renderer {
    pub(super) fn horizontal_size(&mut self, frame: &Frame, m: &Metrics) -> (f32, f32) {
        if frame.rows.is_empty() {
            return (0.0, 0.0);
        }
        let (items, row_height) = self.items(&frame.rows, m);
        let mut width: f32 = items
            .iter()
            .map(|item| item.index_width + m.px(INDEX_GAP) + item.text_width)
            .sum::<f32>()
            + m.column_gap() * items.len().saturating_sub(1) as f32
            + m.px(HIGHLIGHT_INSET) * 2.0;
        if let Some(footer) = frame.footer.as_deref() {
            width += m.column_gap() + self.measure(footer, &m.index_style()).width;
        }
        let mut height = row_height;
        if let Some((annotation_width, annotation_height)) =
            self.highlighted_annotation_size(frame, m)
        {
            width = width.max(annotation_width);
            height += annotation_height;
        }
        (width, height)
    }

    /// 横排时高亮候选的译文行尺寸；高亮候选没有译文时为 `None`。
    fn highlighted_annotation_size(&mut self, frame: &Frame, m: &Metrics) -> Option<(f32, f32)> {
        let row = frame.rows.get(frame.highlighted?)?;
        if row.annotation.is_empty() {
            return None;
        }
        let style = m.annotation_style(m.theme.colors.gloss);
        let width: f32 = row
            .annotation
            .iter()
            .map(|(s, _)| self.measure(s, &style).width)
            .sum();
        Some((width, style.line_height + m.row_padding()))
    }

    /// 横排各项的尺寸与统一行高。
    fn items(&mut self, rows: &[Row], m: &Metrics) -> (Vec<Item>, f32) {
        let mut row_height: f32 = 0.0;
        let text_style = m.text_style();
        let index_style = m.index_style();
        let items = rows
            .iter()
            .map(|row| {
                let index = self.measure(&row.index, &index_style);
                let mut text = self.measure(&row.text, &text_style);
                if row.cloud {
                    text.width += m.cloud_width();
                }
                text.width += self.code_width(row, m);
                row_height = row_height.max(text.height + m.row_padding() * 2.0);
                Item {
                    index_width: index.width,
                    text_width: text.width,
                }
            })
            .collect();
        (items, row_height)
    }

    pub(super) fn draw_horizontal(
        &mut self,
        canvas: &mut Canvas,
        frame: &Frame,
        m: &Metrics,
        left: f32,
        y: f32,
        content_width: f32,
    ) {
        if frame.rows.is_empty() {
            return;
        }
        // 量尺寸时已整形过一遍，这里再整形一遍；等渲染器定型再把结果从 render 传下来。
        let (items, row_height) = self.items(&frame.rows, m);
        let top = y + m.row_padding();
        let text_height = m.px(m.theme.text_font.line_height);
        let inset = m.px(HIGHLIGHT_INSET);
        let mut x = left + m.padding() + inset;
        for (i, (row, item)) in frame.rows.iter().zip(&items).enumerate() {
            let item_width = item.index_width + m.px(INDEX_GAP) + item.text_width;
            if Some(i) == frame.highlighted {
                self.fill_highlight(
                    canvas,
                    m,
                    x - inset,
                    y,
                    item_width + inset * 2.0,
                    row_height,
                );
            }
            self.draw_text(
                canvas,
                &row.index,
                &m.index_style(),
                x,
                top + m.small_offset(text_height),
            );
            self.draw_word(
                canvas,
                m,
                row,
                x + item.index_width + m.px(INDEX_GAP),
                top,
                text_height,
            );
            x += item_width + m.column_gap();
        }
        if let Some(footer) = frame.footer.as_deref() {
            let style = m.index_style();
            let size = self.measure(footer, &style);
            self.draw_text(
                canvas,
                footer,
                &style,
                left + content_width - m.padding() - size.width,
                top + m.small_offset(text_height),
            );
        }
        // 高亮候选的译文
        if let Some(row) = frame.highlighted.and_then(|i| frame.rows.get(i)) {
            let mut x = left + m.padding() + inset;
            let annotation_top = y + row_height + m.row_padding() / 2.0;
            for (segment, tone) in &row.annotation {
                let style = m.annotation_style(m.tone_color(*tone));
                x += self.draw_text(canvas, segment, &style, x, annotation_top);
            }
        }
    }
}
