//! 矩阵：横排展开后的多行网格。一行 `frame.columns` 格，各列宽度由帧给（壳按整份候选估的，滚动时不变），
//! 超宽的候选截尾加「…」；网格下面固定留一行信息：高亮候选被截断时的完整文本、它的译文，页码在行尾。
//! 整个窗口的宽度只由列宽决定，信息行放不下的也截断——高亮怎么移、视口怎么滚，窗口都不跳。

use super::{HIGHLIGHT_INSET, INDEX_GAP, Metrics, Renderer};
use crate::canvas::Canvas;
use crate::frame::Frame;
use crate::text::TextStyle;

/// 帧没给列宽时，一格里候选词最多多宽（按候选字号的倍数）。
const MAX_CELL_EMS: f32 = 4.0;

/// 列宽在估出来的字宽之外再留一点（点）：估宽按整字算，字形实际会多出零点几个像素。
const CELL_SLACK: f32 = 1.5;

/// 信息行里完整文本与译文之间的间距（点）。
const INFO_GAP: f32 = 10.0;

const ELLIPSIS: &str = "…";

/// 量好的网格：每格显示的文字（可能已截断）、各列的宽度与统一的行高（像素）。
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

    /// 网格总宽（不含两侧高亮留边）。
    fn width(&self, gap: f32) -> f32 {
        self.column_widths.iter().sum::<f32>()
            + gap * self.column_widths.len().saturating_sub(1) as f32
    }
}

impl Renderer {
    pub(super) fn matrix_size(&mut self, frame: &Frame, m: &Metrics) -> (f32, f32) {
        if frame.rows.is_empty() {
            return (0.0, 0.0);
        }
        let cells = self.matrix_cells(frame, m);
        let grid_rows = frame.rows.len().div_ceil(frame.columns.max(1));
        let info_height = m.info_height() + m.row_padding();
        (
            cells.width(m.column_gap()) + m.px(HIGHLIGHT_INSET) * 2.0,
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
        let inset = m.px(HIGHLIGHT_INSET);
        let origin = left + m.padding() + inset;
        for (i, row) in frame.rows.iter().enumerate() {
            let text_height = m.word_style(row).line_height;
            let (text, _) = &cells.texts[i];
            if text.is_empty() && row.index.is_empty() {
                continue;
            }
            let cell_width = cells.column_widths[i % columns];
            let x = origin + cells.offset(i % columns, m.column_gap());
            let row_y = y + cells.row_height * (i / columns) as f32;
            let line_height = cells.row_height - m.row_padding() * 2.0;
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
                    top + (line_height - m.index_style().line_height) / 2.0,
                );
            }
            let mut shown = row.clone();
            shown.text.clone_from(text);
            self.draw_word(
                canvas,
                m,
                &shown,
                x + cells.index_width + m.px(INDEX_GAP),
                top + (line_height - text_height) / 2.0,
                text_height,
            );
        }
        // 信息行：页码靠右；左边先放被截断的高亮候选的完整文本，再放译文，放不下的截断
        let grid_rows = frame.rows.len().div_ceil(columns);
        let info_top = y + cells.row_height * grid_rows as f32 + m.row_padding() / 2.0;
        let right = left + content_width - m.padding();
        let mut budget = right - origin;
        if let Some(footer) = frame.footer.as_deref() {
            let style = m.index_style();
            let size = self.measure(footer, &style);
            self.draw_text(
                canvas,
                footer,
                &style,
                right - size.width,
                info_top + (m.info_height() - style.line_height) / 2.0,
            );
            budget -= size.width + m.column_gap();
        }
        let mut x = origin;
        let Some(index) = frame.highlighted else {
            return;
        };
        let Some(row) = frame.rows.get(index) else {
            return;
        };
        if cells.texts[index].1 {
            let used = self.draw_clipped(
                canvas,
                &row.text,
                &m.word_style(row),
                x,
                info_top + (m.info_height() - m.word_style(row).line_height) / 2.0,
                budget,
            );
            x += used + m.px(INFO_GAP);
            budget -= used + m.px(INFO_GAP);
        }
        for (segment, tone) in &row.annotation {
            if budget <= 0.0 {
                break;
            }
            let style = m.segment_style(row, *tone);
            let used = self.draw_clipped(
                canvas,
                segment,
                &style,
                x,
                info_top + (m.info_height() - style.line_height) / 2.0,
                budget,
            );
            x += used;
            budget -= used;
        }
    }

    /// 在信息行里画一段小字，宽度超过 `budget` 就截断；返回画了多宽。
    #[allow(clippy::too_many_arguments)]
    fn draw_clipped(
        &mut self,
        canvas: &mut Canvas,
        text: &str,
        style: &TextStyle,
        x: f32,
        top: f32,
        budget: f32,
    ) -> f32 {
        let (shown, _) = self.truncate(text, style, budget);
        self.draw_text(canvas, &shown, style, x, top)
    }

    /// 每格的显示文字与各列宽度。序号列按最宽的一位数留，行与行才对得齐。
    /// 列宽优先用帧给的（按整份候选估的，滚动时不变）；没给就按视口里实测、每格封顶 [`MAX_CELL_EMS`]。
    fn matrix_cells(&mut self, frame: &Frame, m: &Metrics) -> Cells {
        let columns = frame.columns.max(1);
        let em = m.px(m.theme.text_font.size.max(m.theme.annotation_font.size));
        let index_width = self.measure("8", &m.index_style()).width;
        let fixed: Option<Vec<f32>> = (frame.column_ems.len() == columns).then(|| {
            frame
                .column_ems
                .iter()
                .map(|ems| ems * em + m.px(CELL_SLACK))
                .collect()
        });
        let mut text_widths = fixed.clone().unwrap_or_else(|| vec![0.0; columns]);
        let row_height = frame
            .rows
            .iter()
            .map(|row| m.word_style(row).line_height)
            .fold(m.index_style().line_height, f32::max)
            + m.row_padding() * 2.0;
        let texts = frame
            .rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let text_style = m.word_style(row);
                let column = i % columns;
                let cloud = if row.cloud { m.cloud_width() } else { 0.0 };
                let limit = fixed
                    .as_ref()
                    .map_or(em * MAX_CELL_EMS, |widths| widths[column]);
                // 自定义字族与中英字号可不同，实际测量后再截断，不能只按字符数估计。
                let (text, truncated) = self.truncate(&row.text, &text_style, limit - cloud);
                if fixed.is_none() {
                    let width = self.measure(&text, &text_style).width + cloud;
                    text_widths[column] = text_widths[column].max(width);
                }
                (text, truncated)
            })
            .collect();
        Cells {
            texts,
            index_width,
            column_widths: text_widths
                .iter()
                .map(|width| index_width + m.px(INDEX_GAP) + width)
                .collect(),
            row_height,
        }
    }
}

impl Metrics<'_> {
    /// 网格信息行预留所有文字的最大行高，高亮移动时窗口不跳。
    fn info_height(&self) -> f32 {
        [
            self.theme.text_font,
            self.theme.annotation_font,
            self.theme.pos_font,
            self.theme.index_font,
        ]
        .iter()
        .map(|font| self.px(font.line_height))
        .fold(0.0, f32::max)
    }
}

impl Renderer {
    /// `text` 宽度超过 `max_width` 就从末尾去字、补上「…」直到放得下；返回显示文字与是否截断过。
    fn truncate(&mut self, text: &str, style: &TextStyle, max_width: f32) -> (String, bool) {
        if text.is_empty() || self.measure(text, style).width <= max_width {
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
