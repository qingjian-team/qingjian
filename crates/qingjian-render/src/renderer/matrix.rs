//! 矩阵：横排展开后的多行等宽网格。一行 `frame.columns` 格，超宽的候选截尾加「…」；
//! 网格下面固定留一行信息：高亮候选被截断时的完整文本、它的译文，页码在行尾。

use super::{HIGHLIGHT_INSET, INDEX_GAP, Metrics, Renderer};
use crate::canvas::Canvas;
use crate::frame::Frame;
use crate::text::TextStyle;

/// 一格里候选词最多多宽（按候选字号的倍数）：再长就截断，列宽才不会被一条长句撑开。
const MAX_CELL_EMS: f32 = 6.0;

/// 信息行里完整文本最多多宽（按注释字号的倍数）。
const MAX_INFO_EMS: f32 = 28.0;

/// 信息行里完整文本与译文之间的间距（点）。
const INFO_GAP: f32 = 10.0;

const ELLIPSIS: &str = "…";

/// 量好的网格：每格显示的文字（可能已截断）、各列的宽度（本列最宽的那格）与统一的行高（像素）。
struct Cells {
    texts: Vec<(String, bool)>,

    index_width: f32,

    /// 每列一格的宽度（序号 + 间距 + 候选词）。
    column_widths: Vec<f32>,

    row_height: f32,
}

impl Cells {
    /// 第 `column` 列左边相对网格起点的偏移。
    fn offset(&self, column: usize, gap: f32) -> f32 {
        self.column_widths[..column].iter().sum::<f32>() + gap * column as f32
    }
}

impl Renderer {
    pub(super) fn matrix_size(&mut self, frame: &Frame, m: &Metrics) -> (f32, f32) {
        if frame.rows.is_empty() {
            return (0.0, 0.0);
        }
        let cells = self.matrix_cells(frame, m);
        let columns = frame.columns.max(1);
        let grid_rows = frame.rows.len().div_ceil(columns);
        let grid_width = cells.column_widths.iter().sum::<f32>()
            + m.column_gap() * (columns - 1) as f32
            + m.px(HIGHLIGHT_INSET) * 2.0;
        let (info_width, info_height) = self.info_size(frame, &cells, m);
        (
            grid_width.max(info_width),
            cells.row_height * grid_rows as f32 + info_height,
        )
    }

    pub(super) fn draw_matrix(
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
        let cells = self.matrix_cells(frame, m);
        let columns = frame.columns.max(1);
        let text_height = m.px(m.theme.text_font.line_height);
        let inset = m.px(HIGHLIGHT_INSET);
        let origin = left + m.padding() + inset;
        for (i, row) in frame.rows.iter().enumerate() {
            let (text, _) = &cells.texts[i];
            if text.is_empty() && row.index.is_empty() {
                continue;
            }
            let cell_width = cells.column_widths[i % columns];
            let x = origin + cells.offset(i % columns, m.column_gap());
            let row_y = y + cells.row_height * (i / columns) as f32;
            let top = row_y + m.row_padding();
            if Some(i) == frame.highlighted {
                self.fill_highlight(
                    canvas,
                    m,
                    x - inset,
                    row_y,
                    cell_width + inset * 2.0,
                    cells.row_height,
                );
            }
            if !row.index.is_empty() {
                self.draw_text(
                    canvas,
                    &row.index,
                    &m.index_style(),
                    x,
                    top + m.small_offset(text_height),
                );
            }
            let mut shown = row.clone();
            shown.text.clone_from(text);
            self.draw_word(
                canvas,
                m,
                &shown,
                x + cells.index_width + m.px(INDEX_GAP),
                top,
                text_height,
            );
        }
        // 信息行：被截断的高亮候选给出完整文本，后面是它的译文，页码靠右
        let grid_rows = frame.rows.len().div_ceil(columns);
        let info_top = y + cells.row_height * grid_rows as f32 + m.row_padding() / 2.0;
        let mut x = origin;
        if let Some(full) = self.highlighted_full_text(frame, &cells, m) {
            let style = m.annotation_style(m.theme.colors.text);
            x += self.draw_text(canvas, &full, &style, x, info_top) + m.px(INFO_GAP);
        }
        if let Some(row) = frame.highlighted.and_then(|i| frame.rows.get(i)) {
            for (segment, tone) in &row.annotation {
                let style = m.annotation_style(m.tone_color(*tone));
                x += self.draw_text(canvas, segment, &style, x, info_top);
            }
        }
        if let Some(footer) = frame.footer.as_deref() {
            let style = m.index_style();
            let size = self.measure(footer, &style);
            self.draw_text(
                canvas,
                footer,
                &style,
                left + content_width - m.padding() - size.width,
                info_top,
            );
        }
    }

    /// 每格的显示文字与统一尺寸。序号列按最宽的一位数留，行与行才对得齐。
    fn matrix_cells(&mut self, frame: &Frame, m: &Metrics) -> Cells {
        let text_style = m.text_style();
        let index_width = self.measure("8", &m.index_style()).width;
        let max_text = m.px(m.theme.text_font.size) * MAX_CELL_EMS;
        let columns = frame.columns.max(1);
        let mut column_widths = vec![0.0_f32; columns];
        let mut row_height: f32 = 0.0;
        let texts = frame
            .rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let budget = if row.cloud {
                    max_text - m.cloud_width()
                } else {
                    max_text
                };
                let (text, truncated) = self.truncate(&row.text, &text_style, budget);
                let mut size = self.measure(&text, &text_style);
                if row.cloud {
                    size.width += m.cloud_width();
                }
                let cell = index_width + m.px(INDEX_GAP) + size.width;
                column_widths[i % columns] = column_widths[i % columns].max(cell);
                row_height = row_height.max(size.height + m.row_padding() * 2.0);
                (text, truncated)
            })
            .collect();
        Cells {
            texts,
            index_width,
            column_widths,
            row_height,
        }
    }

    /// 高亮候选被截断时信息行里放的完整文本（本身也有上限）。
    fn highlighted_full_text(
        &mut self,
        frame: &Frame,
        cells: &Cells,
        m: &Metrics,
    ) -> Option<String> {
        let index = frame.highlighted?;
        let (_, truncated) = cells.texts.get(index)?;
        if !truncated {
            return None;
        }
        let style = m.annotation_style(m.theme.colors.text);
        let budget = m.px(m.theme.annotation_font.size) * MAX_INFO_EMS;
        Some(self.truncate(&frame.rows[index].text, &style, budget).0)
    }

    /// 信息行的宽高：矩阵里总留着这一行，高亮来回移动时窗口高度不跳。
    fn info_size(&mut self, frame: &Frame, cells: &Cells, m: &Metrics) -> (f32, f32) {
        let style = m.annotation_style(m.theme.colors.gloss);
        let mut width = m.px(HIGHLIGHT_INSET) * 2.0;
        if let Some(full) = self.highlighted_full_text(frame, cells, m) {
            width += self.measure(&full, &style).width + m.px(INFO_GAP);
        }
        if let Some(row) = frame.highlighted.and_then(|i| frame.rows.get(i)) {
            width += row
                .annotation
                .iter()
                .map(|(s, _)| self.measure(s, &style).width)
                .sum::<f32>();
        }
        if let Some(footer) = frame.footer.as_deref() {
            width += m.column_gap() + self.measure(footer, &m.index_style()).width;
        }
        (width, style.line_height + m.row_padding())
    }

    /// `text` 宽度超过 `max_width` 就从末尾去字、补上「…」直到放得下；返回显示文字与是否截断过。
    fn truncate(&mut self, text: &str, style: &TextStyle, max_width: f32) -> (String, bool) {
        if self.measure(text, style).width <= max_width {
            return (text.to_owned(), false);
        }
        let mut kept: Vec<char> = text.chars().collect();
        while kept.pop().is_some() {
            let mut candidate: String = kept.iter().collect();
            candidate.push_str(ELLIPSIS);
            if kept.is_empty() || self.measure(&candidate, style).width <= max_width {
                return (candidate, true);
            }
        }
        (ELLIPSIS.to_owned(), true)
    }
}
