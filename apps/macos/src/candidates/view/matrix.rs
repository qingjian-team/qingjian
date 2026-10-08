//! 系统绘制路径的矩阵排布：横排展开后的多行网格，规则与渲染器的 `renderer/matrix.rs` 一致——
//! 一行 `frame.columns` 格，各列宽度由帧给（按整份候选估的，滚动时不变），超宽的候选截尾加「…」，
//! 网格下面固定留一行信息（完整文本、译文、页码），信息行放不下的也截断，窗口宽度只由列宽决定。

use crate::candidates::native_font::NativeFont;
use objc2_app_kit::NSColor;
use objc2_foundation::{NSPoint, NSRect, NSSize};

use super::{CandidateView, HIGHLIGHT_INSET, INDEX_GAP};
use crate::candidates::frame::Frame;

/// 帧没给列宽时，一格里候选词最多多宽（按候选字号的倍数）。
const MAX_CELL_EMS: f64 = 4.0;

/// 列宽在估出来的字宽之外再留一点：估宽按整字算，字形实际会多出零点几个点。
const CELL_SLACK: f64 = 1.5;

/// 信息行里完整文本与译文之间的间距。
const INFO_GAP: f64 = 10.0;

const ELLIPSIS: &str = "…";

/// 量好的网格：每格显示的文字（可能已截断）、各列的宽度与统一的行高。
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

    /// 网格总宽（不含两侧高亮留边）。
    fn width(&self, gap: f64) -> f64 {
        self.column_widths.iter().sum::<f64>()
            + gap * self.column_widths.len().saturating_sub(1) as f64
    }
}

impl CandidateView {
    pub(super) fn matrix_size(&self, frame: &Frame) -> (f64, f64) {
        if frame.rows.is_empty() {
            return (0.0, 0.0);
        }
        let theme = self.theme();
        let cells = self.matrix_cells(frame);
        let grid_rows = frame.rows.len().div_ceil(frame.columns.max(1));
        let info_height = self.info_height() + theme.row_padding;
        (
            cells.width(theme.column_gap) + HIGHLIGHT_INSET * 2.0,
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
        let origin = theme.padding + HIGHLIGHT_INSET;
        for (i, row) in frame.rows.iter().enumerate() {
            let text_height = self.measure(&row.text, theme.word_font(row)).height;
            let (text, _) = &cells.texts[i];
            if text.is_empty() && row.index.is_empty() {
                continue;
            }
            let cell_width = cells.column_widths[i % columns];
            let x = origin + cells.offset(i % columns, theme.column_gap);
            let row_y = y + cells.row_height * (i / columns) as f64;
            let baseline = row_y + theme.row_padding;
            let line_height = cells.row_height - theme.row_padding * 2.0;
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
                    baseline
                        + (line_height - self.measure(&row.index, &theme.index_font).height) / 2.0,
                    x,
                );
            }
            let mut shown = row.clone();
            shown.text.clone_from(text);
            self.draw_word(
                &shown,
                x + cells.index_width + INDEX_GAP,
                baseline + (line_height - text_height) / 2.0,
                text_height,
            );
        }
        // 信息行：页码靠右；左边先放被截断的高亮候选的完整文本，再放译文，放不下的截断
        let grid_rows = frame.rows.len().div_ceil(columns);
        let info_top = y + cells.row_height * grid_rows as f64 + theme.row_padding / 2.0;
        let right = bounds.size.width - theme.padding;
        let mut budget = right - origin;
        if let Some(footer) = frame.footer.as_deref() {
            let size = self.measure(footer, &theme.index_font);
            self.draw_text(
                footer,
                &theme.index_font,
                &theme.index_color,
                info_top + (self.info_height() - size.height) / 2.0,
                right - size.width,
            );
            budget -= size.width + theme.column_gap;
        }
        let Some(row) = frame.rows.get(frame.highlighted) else {
            return;
        };
        let mut x = origin;
        if cells.texts[frame.highlighted].1 {
            let used = self.draw_clipped(
                &row.text,
                theme.word_font(row),
                &theme.text_color,
                x,
                info_top,
                budget,
            );
            x += used + INFO_GAP;
            budget -= used + INFO_GAP;
        }
        for (segment, tone) in &row.annotation {
            if budget <= 0.0 {
                break;
            }
            let used = self.draw_clipped(
                segment,
                theme.segment_font(row, *tone),
                &self.tone_color(*tone),
                x,
                info_top,
                budget,
            );
            x += used;
            budget -= used;
        }
    }

    /// 在信息行里画一段小字，宽度超过 `budget` 就截断；返回画了多宽。
    fn draw_clipped(
        &self,
        text: &str,
        font: &NativeFont,
        color: &NSColor,
        x: f64,
        top: f64,
        budget: f64,
    ) -> f64 {
        let (shown, _) = self.truncate(text, font, budget);
        self.draw_text(
            &shown,
            font,
            color,
            top + (self.info_height() - self.measure(&shown, font).height) / 2.0,
            x,
        )
    }

    fn info_height(&self) -> f64 {
        let theme = self.theme();
        [
            &theme.text_font,
            &theme.annotation_font,
            &theme.pos_font,
            &theme.index_font,
        ]
        .iter()
        .map(|font| self.measure("Ag国", font).height)
        .fold(0.0, f64::max)
    }

    /// 每格的显示文字与各列宽度。序号列按最宽的一位数留，行与行才对得齐。
    /// 列宽优先用帧给的（按整份候选估的，滚动时不变）；没给就按视口里实测、每格封顶 [`MAX_CELL_EMS`]。
    fn matrix_cells(&self, frame: &Frame) -> Cells {
        let theme = self.theme();
        let columns = frame.columns.max(1);
        let em = theme
            .text_font
            .pointSize()
            .max(theme.annotation_font.pointSize());
        let index_width = self.measure("8", &theme.index_font).width;
        let fixed: Option<Vec<f64>> = (frame.column_ems.len() == columns).then(|| {
            frame
                .column_ems
                .iter()
                .map(|ems| f64::from(*ems) * em + CELL_SLACK)
                .collect()
        });
        let mut text_widths = fixed.clone().unwrap_or_else(|| vec![0.0; columns]);
        let mut row_height: f64 = 0.0;
        let texts = frame
            .rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let font = theme.word_font(row);
                let column = i % columns;
                let cloud = if row.cloud { self.cloud_width() } else { 0.0 };
                let limit = fixed
                    .as_ref()
                    .map_or(em * MAX_CELL_EMS, |widths| widths[column]);
                let (text, truncated) = self.truncate(&row.text, font, limit - cloud);
                let size = self.measure(&text, font);
                if fixed.is_none() {
                    text_widths[column] = text_widths[column].max(size.width + cloud);
                }
                row_height = row_height.max(
                    size.height.max(self.measure("8", &theme.index_font).height)
                        + theme.row_padding * 2.0,
                );
                (text, truncated)
            })
            .collect();
        Cells {
            texts,
            index_width,
            column_widths: text_widths
                .iter()
                .map(|width| index_width + INDEX_GAP + width)
                .collect(),
            row_height,
        }
    }

    /// `text` 宽度超过 `max_width` 就从末尾去字、补上「…」直到放得下；返回显示文字与是否截断过。
    fn truncate(&self, text: &str, font: &NativeFont, max_width: f64) -> (String, bool) {
        if text.is_empty() || self.measure(text, font).width <= max_width {
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
