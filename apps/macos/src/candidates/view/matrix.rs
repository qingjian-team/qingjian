//! 系统绘制路径的矩阵排布：横排展开后的多行等宽网格，规则与渲染器的 `renderer/matrix.rs` 一致——
//! 一行 `frame.columns` 格，超宽的候选截尾加「…」，网格下面固定留一行信息（完整文本、译文、页码）。

use objc2_app_kit::NSFont;
use objc2_foundation::{NSPoint, NSRect, NSSize};

use super::{CandidateView, HIGHLIGHT_INSET, INDEX_GAP};
use crate::candidates::frame::Frame;

/// 一格里候选词最多多宽（按候选字号的倍数）。
const MAX_CELL_EMS: f64 = 6.0;

/// 信息行里完整文本最多多宽（按注释字号的倍数）。
const MAX_INFO_EMS: f64 = 28.0;

/// 信息行里完整文本与译文之间的间距。
const INFO_GAP: f64 = 10.0;

const ELLIPSIS: &str = "…";

/// 量好的网格：每格显示的文字（可能已截断）、各列的宽度（本列最宽的那格）与统一的行高。
struct Cells {
    texts: Vec<(String, bool)>,

    index_width: f64,

    /// 每列一格的宽度（序号 + 间距 + 候选词）。
    column_widths: Vec<f64>,

    row_height: f64,
}

impl Cells {
    /// 第 `column` 列左边相对网格起点的偏移。
    fn offset(&self, column: usize, gap: f64) -> f64 {
        self.column_widths[..column].iter().sum::<f64>() + gap * column as f64
    }
}

impl CandidateView {
    pub(super) fn matrix_size(&self, frame: &Frame) -> (f64, f64) {
        if frame.rows.is_empty() {
            return (0.0, 0.0);
        }
        let theme = self.theme();
        let cells = self.matrix_cells(frame);
        let columns = frame.columns.max(1);
        let grid_rows = frame.rows.len().div_ceil(columns);
        let grid_width = cells.column_widths.iter().sum::<f64>()
            + theme.column_gap * (columns - 1) as f64
            + HIGHLIGHT_INSET * 2.0;
        let (info_width, info_height) = self.info_size(frame, &cells);
        (
            grid_width.max(info_width),
            cells.row_height * grid_rows as f64 + info_height,
        )
    }

    pub(super) fn draw_matrix(&self, frame: &Frame, y: f64, bounds: NSRect) {
        if frame.rows.is_empty() {
            return;
        }
        let theme = self.theme();
        let cells = self.matrix_cells(frame);
        let columns = frame.columns.max(1);
        let text_height = self.measure("x", &theme.text_font).height;
        let origin = theme.padding + HIGHLIGHT_INSET;
        for (i, row) in frame.rows.iter().enumerate() {
            let (text, _) = &cells.texts[i];
            if text.is_empty() && row.index.is_empty() {
                continue;
            }
            let cell_width = cells.column_widths[i % columns];
            let x = origin + cells.offset(i % columns, theme.column_gap);
            let row_y = y + cells.row_height * (i / columns) as f64;
            let baseline = row_y + theme.row_padding;
            if i == frame.highlighted {
                self.fill_highlight(NSRect::new(
                    NSPoint::new(x - HIGHLIGHT_INSET, row_y),
                    NSSize::new(cell_width + HIGHLIGHT_INSET * 2.0, cells.row_height),
                ));
            }
            if !row.index.is_empty() {
                self.draw_text(
                    &row.index,
                    &theme.index_font,
                    &theme.index_color,
                    baseline + self.small_offset(text_height),
                    x,
                );
            }
            let mut shown = row.clone();
            shown.text.clone_from(text);
            self.draw_word(
                &shown,
                x + cells.index_width + INDEX_GAP,
                baseline,
                text_height,
            );
        }
        // 信息行：被截断的高亮候选给出完整文本，后面是它的译文，页码靠右
        let grid_rows = frame.rows.len().div_ceil(columns);
        let info_top = y + cells.row_height * grid_rows as f64 + theme.row_padding / 2.0;
        let mut x = origin;
        if let Some(full) = self.highlighted_full_text(frame, &cells) {
            x += self.draw_text(
                &full,
                &theme.annotation_font,
                &theme.text_color,
                info_top,
                x,
            ) + INFO_GAP;
        }
        if let Some(row) = frame.rows.get(frame.highlighted) {
            for (segment, tone) in &row.annotation {
                x += self.draw_text(
                    segment,
                    &theme.annotation_font,
                    self.tone_color(*tone),
                    info_top,
                    x,
                );
            }
        }
        if let Some(footer) = frame.footer.as_deref() {
            let size = self.measure(footer, &theme.index_font);
            self.draw_text(
                footer,
                &theme.index_font,
                &theme.index_color,
                info_top,
                bounds.size.width - theme.padding - size.width,
            );
        }
    }

    /// 每格的显示文字与统一尺寸。序号列按最宽的一位数留，行与行才对得齐。
    fn matrix_cells(&self, frame: &Frame) -> Cells {
        let theme = self.theme();
        let index_width = self.measure("8", &theme.index_font).width;
        let max_text = theme.text_font.pointSize() * MAX_CELL_EMS;
        let columns = frame.columns.max(1);
        let mut column_widths = vec![0.0_f64; columns];
        let mut row_height: f64 = 0.0;
        let texts = frame
            .rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let budget = if row.cloud {
                    max_text - self.cloud_width()
                } else {
                    max_text
                };
                let (text, truncated) = self.truncate(&row.text, &theme.text_font, budget);
                let size = self.measure(&text, &theme.text_font);
                let width = if row.cloud {
                    size.width + self.cloud_width()
                } else {
                    size.width
                };
                let cell = index_width + INDEX_GAP + width;
                column_widths[i % columns] = column_widths[i % columns].max(cell);
                row_height = row_height.max(size.height + theme.row_padding * 2.0);
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
    fn highlighted_full_text(&self, frame: &Frame, cells: &Cells) -> Option<String> {
        let (_, truncated) = cells.texts.get(frame.highlighted)?;
        if !truncated {
            return None;
        }
        let font = &self.theme().annotation_font;
        let budget = font.pointSize() * MAX_INFO_EMS;
        Some(
            self.truncate(&frame.rows[frame.highlighted].text, font, budget)
                .0,
        )
    }

    /// 信息行的宽高：矩阵里总留着这一行，高亮来回移动时窗口高度不跳。
    fn info_size(&self, frame: &Frame, cells: &Cells) -> (f64, f64) {
        let theme = self.theme();
        let mut width = HIGHLIGHT_INSET * 2.0;
        if let Some(full) = self.highlighted_full_text(frame, cells) {
            width += self.measure(&full, &theme.annotation_font).width + INFO_GAP;
        }
        if let Some(row) = frame.rows.get(frame.highlighted) {
            width += row
                .annotation
                .iter()
                .map(|(s, _)| self.measure(s, &theme.annotation_font).width)
                .sum::<f64>();
        }
        if let Some(footer) = frame.footer.as_deref() {
            width += theme.column_gap + self.measure(footer, &theme.index_font).width;
        }
        let height = self.measure("x", &theme.annotation_font).height + theme.row_padding;
        (width, height)
    }

    /// `text` 宽度超过 `max_width` 就从末尾去字、补上「…」直到放得下；返回显示文字与是否截断过。
    fn truncate(&self, text: &str, font: &NSFont, max_width: f64) -> (String, bool) {
        if self.measure(text, font).width <= max_width {
            return (text.to_owned(), false);
        }
        let mut kept: Vec<char> = text.chars().collect();
        while kept.pop().is_some() {
            let mut candidate: String = kept.iter().collect();
            candidate.push_str(ELLIPSIS);
            if kept.is_empty() || self.measure(&candidate, font).width <= max_width {
                return (candidate, true);
            }
        }
        (ELLIPSIS.to_owned(), true)
    }
}
